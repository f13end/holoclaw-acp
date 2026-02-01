# Security Considerations for Holochain-OpenClaw Hybrid

## Overview

This document outlines critical security considerations for the hybrid Holochain + Base L2 implementation.

## 1. Private Key Storage

### Problem
The system requires two private keys:
- **Base L2 Wallet Private Key**: For escrow transactions and payments
- **Holochain Agent Key**: For signing DHT entries (managed by conductor)

### Solutions

#### Option A: TEE (Trusted Execution Environment)
**Recommended for Production**

Use hardware-backed secure enclaves (Intel SGX, ARM TrustZone, AWS Nitro):

```rust
// Pseudocode: Load keys from TEE
use sgx_tcrypto::*;

fn get_wallet_key_from_tee() -> Result<[u8; 32], Error> {
    let sealed_key = sgx_read_sealed_data("wallet_key")?;
    let unsealed = sgx_unseal_data(sealed_key)?;
    Ok(unsealed)
}
```

**Advantages:**
- Keys never exposed to host OS
- Hardware-level isolation
- Attestation for verification

**Implementation:**
1. Run Holochain conductor in TEE enclave
2. Store Base wallet key in SGX sealed storage
3. Use remote attestation for agent verification

#### Option B: Local Encrypted Keystore
**Good for Development/Testing**

Use OS keychain with encryption:

```typescript
import keytar from 'keytar';

// Store
await keytar.setPassword('holoclaw-acp', 'wallet_key', privateKey);

// Retrieve
const key = await keytar.getPassword('holoclaw-acp', 'wallet_key');
```

**Linux:** Use gnome-keyring or KWallet  
**macOS:** Use Keychain  
**Windows:** Use Credential Manager

#### Option C: Environment Variables with Encryption
**Minimum Viable**

Encrypt env vars with master password:

```bash
# Encrypt
echo $WALLET_PRIVATE_KEY | openssl enc -aes-256-cbc -pbkdf2 -out wallet.enc

# Decrypt at runtime
export WALLET_PRIVATE_KEY=$(openssl enc -d -aes-256-cbc -pbkdf2 -in wallet.enc)
```

**⚠️ Warning:** Not secure against memory dumps or process inspection.

### Best Practices

1. **Never log private keys** - sanitize all logs
2. **Use capability grants** - limit key access to specific functions
3. **Rotate keys periodically** - implement key rotation strategy
4. **Audit key access** - log all key usage (not the key itself)
5. **Separate concerns** - use different keys for testing vs production

## 2. Membrane Proofs

### Purpose
Control who can join the Holochain DHT and prevent sybil attacks.

### Implementation

#### Simple Whitelist Membrane

```rust
// In integrity zome
#[derive(Serialize, Deserialize, Debug)]
pub struct WhitelistProof {
    pub wallet_address: String,
    pub signature: Vec<u8>,
}

#[hdk_extern]
pub fn validate_agent(
    _agent_pub_key: AgentPubKey,
    membrane_proof: Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    let proof = membrane_proof.ok_or(wasm_error!(
        "Membrane proof required"
    ))?;

    let whitelist_proof: WhitelistProof = proof.try_into()?;

    // Check if wallet is whitelisted (query external service or hardcoded list)
    let is_whitelisted = check_whitelist(&whitelist_proof.wallet_address)?;
    
    if !is_whitelisted {
        return Ok(ValidateCallbackResult::Invalid(
            "Wallet not whitelisted".into()
        ));
    }

    // Verify signature proves ownership of wallet
    let is_valid_sig = verify_eth_signature(
        &whitelist_proof.wallet_address,
        &whitelist_proof.signature,
    )?;

    if !is_valid_sig {
        return Ok(ValidateCallbackResult::Invalid(
            "Invalid signature".into()
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
```

#### Stake-Based Membrane

Require proof of ACP stake or token holdings:

```rust
pub struct StakeProof {
    pub wallet_address: String,
    pub stake_amount: u64,
    pub block_height: u64,
    pub merkle_proof: Vec<u8>,
}

fn validate_stake(proof: &StakeProof) -> ExternResult<bool> {
    // Verify merkle proof against on-chain state root
    // Ensure stake_amount meets minimum threshold
    // Ensure block_height is recent
    Ok(true) // Placeholder
}
```

### Joining Flow

1. **User registers on ACP** → gets whitelisted wallet
2. **Sign membrane proof** with wallet private key
3. **Submit proof** when installing Holochain DNA
4. **Conductor validates** proof before creating agent

```typescript
// Client-side joining
import { ethers } from 'ethers';

const wallet = new ethers.Wallet(privateKey);
const message = `Join Holoclaw ACP at ${Date.now()}`;
const signature = await wallet.signMessage(message);

await client.installApp({
  agent_key: agentPubKey,
  membrane_proof: {
    wallet_address: wallet.address,
    signature: ethers.getBytes(signature),
  },
});
```

## 3. Async Bridge Pattern

### Problem
Holochain zomes run in WASM and cannot block on I/O. External calls (Base RPC, HTTP) must be async.

### Solution: Signal-Based Pattern

#### Architecture

```
┌────────────┐         ┌─────────────────┐         ┌──────────────┐
│ Coordinator│ signal  │  OpenClaw       │   HTTP  │  Base RPC    │
│   Zome     │────────>│  Runtime        │────────>│  Provider    │
│            │<────────│  (TypeScript)   │<────────│              │
└────────────┘  call   └─────────────────┘         └──────────────┘
```

#### Implementation

**Step 1: Emit signal from coordinator zome**

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BalanceRequest {
    pub request_id: String,
    pub address: String,
}

#[hdk_extern]
pub fn request_wallet_balance(address: String) -> ExternResult<String> {
    let request_id = format!("balance_{}", sys_time()?.as_micros());
    
    emit_signal(SignalPayload::BalanceRequest(BalanceRequest {
        request_id: request_id.clone(),
        address,
    }))?;
    
    // Return request_id for tracking
    Ok(request_id)
}
```

**Step 2: Handle signal in OpenClaw**

```typescript
client.on('signal', async (signal) => {
  if (signal.data.type === 'BalanceRequest') {
    const { request_id, address } = signal.data;
    
    // Query Base RPC
    const balance = await provider.getBalance(address);
    
    // Call back into Holochain with result
    await client.callZome({
      zome_name: 'acp_coordinator',
      fn_name: 'handle_balance_response',
      payload: {
        request_id,
        balance: balance.toString(),
      },
    });
  }
});
```

**Step 3: Store response in zome**

```rust
use std::collections::HashMap;

// In-memory cache (lost on restart - use source chain for persistence)
static mut BALANCE_CACHE: Option<HashMap<String, String>> = None;

#[hdk_extern]
pub fn handle_balance_response(
    request_id: String,
    balance: String,
) -> ExternResult<()> {
    unsafe {
        if BALANCE_CACHE.is_none() {
            BALANCE_CACHE = Some(HashMap::new());
        }
        BALANCE_CACHE.as_mut().unwrap().insert(request_id, balance);
    }
    Ok(())
}
```

### Benefits
- Non-blocking zome execution
- Composable async operations
- Clean separation of concerns

### Limitations
- Request/response tracking complexity
- No automatic retry on failure
- State management challenges

## 4. Additional Security Measures

### Rate Limiting

Prevent DHT abuse:

```rust
#[hdk_extern]
pub fn browse_agents(query: BrowseAgentsQuery) -> ExternResult<Vec<AcpAgentInfo>> {
    // Check rate limit (store timestamps in source chain)
    let last_call = get_last_browse_timestamp()?;
    let now = sys_time()?.as_micros();
    
    if now - last_call < 1_000_000 { // 1 second cooldown
        return Err(wasm_error!("Rate limit exceeded"));
    }
    
    store_browse_timestamp(now)?;
    
    // Continue with function...
}
```

### Input Validation

Always validate external inputs:

```rust
fn validate_ethereum_address(addr: &str) -> ExternResult<()> {
    if !addr.starts_with("0x") || addr.len() != 42 {
        return Err(wasm_error!("Invalid Ethereum address"));
    }
    
    // Additional checksum validation
    if !is_valid_checksum(addr)? {
        return Err(wasm_error!("Invalid address checksum"));
    }
    
    Ok(())
}
```

### Capability Grants

Restrict access to sensitive functions:

```rust
#[hdk_extern]
pub fn create_grant_for_balance_check() -> ExternResult<()> {
    let grant = ZomeCallCapGrant {
        tag: "balance_checker".into(),
        access: CapAccess::Assigned {
            secret: CapSecret::from([1u8; 64]),
            assignees: BTreeSet::from([agent_info()?.agent_initial_pubkey]),
        },
        functions: GrantedFunctions::Listed(vec![
            ("acp_coordinator".into(), "get_wallet_balance".into()),
        ]),
    };
    
    create_cap_grant(grant)?;
    Ok(())
}
```

### Audit Logging

Log all sensitive operations to source chain:

```rust
#[hdk_entry_helper]
pub struct AuditLog {
    pub timestamp: u64,
    pub action: String,
    pub agent: AgentPubKey,
    pub details: String,
}

fn log_audit(action: &str, details: &str) -> ExternResult<()> {
    let log = AuditLog {
        timestamp: sys_time()?.as_micros(),
        action: action.to_string(),
        agent: agent_info()?.agent_initial_pubkey,
        details: details.to_string(),
    };
    
    create_entry(EntryTypes::AuditLog(log))?;
    Ok(())
}
```

## 5. Security Checklist

Before deploying to production:

- [ ] Private keys stored in TEE or encrypted keystore
- [ ] Membrane proof validates wallet ownership
- [ ] Rate limiting on all public functions
- [ ] Input validation on all external data
- [ ] Capability grants for sensitive operations
- [ ] Audit logging enabled
- [ ] HTTPS/WSS for all network communication
- [ ] Regular security audits scheduled
- [ ] Incident response plan documented
- [ ] Key rotation procedure established
- [ ] Backup and recovery tested
- [ ] Access control reviewed
- [ ] Dependencies scanned for vulnerabilities
- [ ] Penetration testing completed

## 6. Threat Model

### Threats Mitigated
✅ Unauthorized DHT access (membrane proofs)  
✅ Key exposure (TEE/keystore)  
✅ DHT spam (rate limiting)  
✅ Invalid data injection (validation rules)  
✅ Replay attacks (timestamps, nonces)

### Threats NOT Mitigated (Require Additional Work)
⚠️ DDoS on conductor endpoint (needs network-level protection)  
⚠️ Sophisticated sybil attacks (needs stake requirements)  
⚠️ Compromised OpenClaw runtime (needs sandboxing)  
⚠️ Base L2 MEV attacks (needs private mempool)  
⚠️ Social engineering (needs user education)

## References

- [Holochain Security Best Practices](https://developer.holochain.org/concepts/4_security/)
- [Ethereum Key Management](https://consensys.github.io/smart-contract-best-practices/tokens/key-management/)
- [TEE Programming Guide](https://sgx101.gitbook.io/sgx101/)
