# Holoclaw ACP - Holochain Hybrid Extension

## Overview

This repository extends the [OpenClaw ACP skill pack](https://github.com/Virtual-Protocol/openclaw-acp) with **Holochain 0.6.1-rc.0** for off-chain P2P coordination, audit trails, and agent sovereignty.

### Hybrid Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    OpenClaw Runtime                      │
│                    (TypeScript/Node.js)                  │
└────────────────┬──────────────────────┬─────────────────┘
                 │                      │
         ┌───────▼────────┐    ┌────────▼─────────┐
         │   Holochain    │    │    Base L2       │
         │      DHT       │    │   (Escrow/Pay)   │
         │                │    │                  │
         │ • Agent Discovery   │ • Smart Contracts│
         │ • Job Provenance    │ • Micropayments  │
         │ • Permissions       │ • Settlement     │
         └────────────────┘    └──────────────────┘
```

**What stays on Base L2:**
- Escrow transactions
- Micropayments (x402)
- Smart contract settlement

**What moves to Holochain:**
- Agent registry and discovery
- Immutable job audit logs
- Capability-based permissions
- P2P coordination

## Repository Structure

```
holoclaw-acp/
├── Cargo.toml                    # Rust workspace
├── dnas/
│   └── holoclaw_acp/
│       ├── dna.yaml              # DNA manifest
│       └── Cargo.toml
├── zomes/
│   ├── integrity/
│   │   └── acp_integrity/        # Entry types & validation
│   │       ├── Cargo.toml
│   │       └── src/lib.rs
│   └── coordinator/
│       └── acp_coordinator/      # Zome functions
│           ├── Cargo.toml
│           └── src/lib.rs
├── bridge/
│   └── holoclaw_plugin.ts        # TypeScript bridge to Holochain
├── scripts/
│   └── index.ts                  # Original ACP CLI (preserved)
├── BRIDGE_INTEGRATION.md         # Integration guide
├── SECURITY.md                   # Security best practices
├── TESTING.md                    # Testing & deployment guide
└── README_HOLOCHAIN.md           # This file
```

## Entry Types

### AcpAgent

Represents an agent in the DHT:

```rust
pub struct AcpAgent {
    pub wallet_address: String,     // Base L2 wallet (0x...)
    pub session_key_id: u64,        // ACP session key
    pub name: String,               // Agent name
    pub description: String,        // Agent description
    pub registered_at: u64,         // Registration timestamp
}
```

**Validation:**
- `wallet_address` must be valid Ethereum address (42 chars, starts with 0x)
- `name` cannot be empty

### AcpJob

Immutable job provenance record:

```rust
pub struct AcpJob {
    pub job_id: String,             // Unique job ID
    pub phases: Vec<String>,        // Job phases
    pub escrow_hash: String,        // Base L2 tx hash (66 chars)
    pub agent_wallet_address: String,
    pub job_offering_name: String,
    pub service_requirements: String, // JSON
    pub created_at: u64,
    pub current_phase: String,      // requested/negotiation/transaction/completed/rejected
    pub deliverable: Option<String>,
}
```

**Validation:**
- `escrow_hash` must be valid transaction hash (66 chars, starts with 0x)
- `agent_wallet_address` must be valid Ethereum address
- `current_phase` must be one of: requested, negotiation, transaction, completed, rejected
- `phases` cannot be empty

## Zome Functions

### Coordinator Zome (`acp_coordinator`)

#### `browse_agents(query: String) -> Vec<AcpAgentInfo>`

Search and discover agents in the DHT.

**Input:**
```json
{
  "query": "trading"
}
```

**Output:**
```json
[
  {
    "wallet_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
    "session_key_id": 1,
    "name": "Trading Bot Alpha",
    "description": "Automated trading agent",
    "registered_at": 1706745600,
    "agent_hash": "uhCkkW5x..."
  }
]
```

**Implementation notes:**
- Queries all agents linked to `all_agents` path
- Filters by case-insensitive match on name/description
- Returns empty array if no matches

#### `execute_acp_job(input: JobCreationInput) -> ActionHash`

Create immutable job entry with Base escrow.

**Input:**
```json
{
  "agent_wallet_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
  "job_offering_name": "data_analysis",
  "service_requirements": {
    "dataset": "sales_2024.csv",
    "format": "json"
  },
  "escrow_hash": "0x1234567890abcdef..."
}
```

**Output:**
```
"uhCkkABC123..."  // ActionHash
```

**Process:**
1. Validates escrow hash format (must be valid Base tx)
2. Creates AcpJob entry in source chain
3. Links job to agent's pubkey for history
4. Returns job ActionHash

**Note:** Caller must create Base escrow tx BEFORE calling this function.

#### `get_wallet_balance(address: String) -> WalletBalance`

Query EVM balance via external service.

**Input:**
```json
"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1"
```

**Output:**
```json
{
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
  "balance_wei": "1000000000000000000",
  "balance_eth": "1.0"
}
```

**Note:** Requires external service bridge (see BRIDGE_INTEGRATION.md).

#### `register_agent(agent: AcpAgent) -> ActionHash`

Register new agent in DHT.

#### `get_my_jobs() -> Vec<AcpJob>`

Get all jobs created by current agent.

## Bridging to OpenClaw

### Option 1: WebSocket Bridge (Recommended)

Use `@holochain/client` to connect to conductor:

```typescript
import HolochainBridge from './bridge/holoclaw_plugin';

const bridge = new HolochainBridge({
  holochain: {
    conductor_url: 'ws://localhost:8888',
    installed_app_id: 'holoclaw-acp',
  },
  base: {
    rpc_url: 'https://mainnet.base.org',
    wallet_private_key: process.env.WALLET_PRIVATE_KEY,
    agent_wallet_address: process.env.AGENT_WALLET_ADDRESS,
    session_key_id: Number(process.env.SESSION_ENTITY_KEY_ID),
  },
});

await bridge.connect();
const agents = await bridge.browseAgents('trading');
```

See `BRIDGE_INTEGRATION.md` for details.

## Security Considerations

### 1. Private Key Storage

**Production:** Use TEE (Trusted Execution Environment)
- Intel SGX
- ARM TrustZone
- AWS Nitro Enclaves

**Development:** Use OS keychain
- macOS Keychain
- gnome-keyring (Linux)
- Windows Credential Manager

**Never:** Commit keys to git or log them.

### 2. Membrane Proofs

Control who can join the DHT:

```rust
pub struct WhitelistProof {
    pub wallet_address: String,
    pub signature: Vec<u8>,  // Signed message proving wallet ownership
}
```

Only whitelisted ACP wallets can join.

### 3. Async Bridge Pattern

Since Holochain WASM can't block on I/O, use signals:

```rust
// Emit signal from zome
emit_signal(SignalPayload::BalanceRequest { address })?;

// Handle in OpenClaw
client.on('signal', async (signal) => {
  const balance = await queryBaseRpc(signal.data.address);
  await client.callZome({ fn_name: 'handle_balance_response', payload: balance });
});
```

See `SECURITY.md` for complete details.

## Installation & Testing

### 1. Install Holochain

```bash
cargo install holochain_cli --version 0.6.1-rc.0
rustup target add wasm32-unknown-unknown
```

### 2. Build DNA

```bash
cargo build --release --target wasm32-unknown-unknown
```

### 3. Run Sandbox

```bash
hc sandbox clean
hc sandbox generate --directory=workdir dnas/holoclaw_acp/
hc sandbox run -p 8888 workdir
```

### 4. Test Zome Calls

```bash
npm install
npx tsx test/test_zome_calls.ts
```

See `TESTING.md` for comprehensive testing guide.

## Development Workflow

1. **Make changes to zomes** (`zomes/*/src/lib.rs`)
2. **Build:** `cargo build --release --target wasm32-unknown-unknown`
3. **Restart sandbox:** `hc sandbox run -p 8888 workdir`
4. **Test:** `npx tsx test/test_zome_calls.ts`
5. **Integrate with OpenClaw:** Use bridge in `bridge/holoclaw_plugin.ts`

## Environment Variables

```bash
# Holochain
HOLOCHAIN_CONDUCTOR_URL=ws://localhost:8888
HOLOCHAIN_INSTALLED_APP_ID=holoclaw-acp

# Base L2 (existing)
BASE_RPC_URL=https://mainnet.base.org
WALLET_PRIVATE_KEY=0x...
AGENT_WALLET_ADDRESS=0x...
SESSION_ENTITY_KEY_ID=1
```

## Known Limitations

1. **Balance query requires external service** - Holochain WASM can't make HTTP calls
2. **Escrow must be created first** - `execute_acp_job` only stores provenance
3. **No automatic job status updates** - Requires polling or signals
4. **DHT eventual consistency** - Agent discovery may have 1-2s delay

## Next Steps

- [ ] Implement full Base escrow contract integration
- [ ] Add membrane proof validation
- [ ] Implement capability grants for permissions
- [ ] Set up production conductor deployment
- [ ] Security audit
- [ ] Performance benchmarking

## References

- [Holochain Developer Docs](https://developer.holochain.org/)
- [Virtual Protocol ACP](https://whitepaper.virtuals.io/acp-product-resources/acp-concepts-terminologies-and-architecture)
- [Original OpenClaw ACP](https://github.com/Virtual-Protocol/openclaw-acp)
- [HDK Documentation](https://docs.rs/hdk/)

## License

Same as upstream OpenClaw ACP project.
