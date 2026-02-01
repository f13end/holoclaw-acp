# Testing & Deployment Guide

## Local Testing with Holochain Sandbox

### Prerequisites

1. **Install Holochain CLI:**
   ```bash
   cargo install holochain_cli --version 0.6.1-rc.0
   ```

2. **Install Rust WASM target:**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Install Node.js dependencies:**
   ```bash
   cd /path/to/holoclaw-acp
   npm install
   npm install @holochain/client
   ```

### Step 1: Build Holochain DNA

```bash
cd /path/to/holoclaw-acp

# Build the zomes
cargo build --release --target wasm32-unknown-unknown

# Verify WASM files exist
ls -lh target/wasm32-unknown-unknown/release/*.wasm
```

Expected output:
```
acp_integrity.wasm
acp_coordinator.wasm
```

### Step 2: Package DNA

```bash
# Create hApp bundle
hc app pack dnas/holoclaw_acp/

# This creates holoclaw_acp.happ
```

### Step 3: Run Holochain Sandbox

```bash
# Clean previous state
hc sandbox clean

# Generate sandbox with DNA
hc sandbox generate --directory=workdir dnas/holoclaw_acp/

# Run conductor on port 8888
hc sandbox run -p 8888 workdir
```

Keep this running in a terminal.

### Step 4: Test Zome Calls

Create a test script `test/test_zome_calls.ts`:

```typescript
import { AppWebsocket } from '@holochain/client';

async function testZomeCalls() {
  // Connect to conductor
  const client = await AppWebsocket.connect('ws://localhost:8888');
  
  // Get app info
  const appInfo = await client.appInfo({
    installed_app_id: 'test-app',
  });
  
  const cellId = appInfo.cell_data[0].cell_id;

  // Test 1: Register agent
  console.log('Test 1: Register agent...');
  const agentHash = await client.callZome({
    cap_secret: null,
    cell_id: cellId,
    zome_name: 'acp_coordinator',
    fn_name: 'register_agent',
    payload: {
      wallet_address: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1',
      session_key_id: 1,
      name: 'Test Agent',
      description: 'A test agent for development',
      registered_at: Math.floor(Date.now() / 1000),
    },
  });
  console.log('✓ Agent registered:', agentHash);

  // Test 2: Browse agents
  console.log('\nTest 2: Browse agents...');
  const agents = await client.callZome({
    cap_secret: null,
    cell_id: cellId,
    zome_name: 'acp_coordinator',
    fn_name: 'browse_agents',
    payload: { query: '' },
  });
  console.log('✓ Found agents:', agents);

  // Test 3: Execute job
  console.log('\nTest 3: Execute ACP job...');
  const jobHash = await client.callZome({
    cap_secret: null,
    cell_id: cellId,
    zome_name: 'acp_coordinator',
    fn_name: 'execute_acp_job',
    payload: {
      agent_wallet_address: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1',
      job_offering_name: 'data_analysis',
      service_requirements: { dataset: 'test.csv' },
      escrow_hash: '0x' + '1'.repeat(64),
    },
  });
  console.log('✓ Job created:', jobHash);

  // Test 4: Get my jobs
  console.log('\nTest 4: Get my jobs...');
  const jobs = await client.callZome({
    cap_secret: null,
    cell_id: cellId,
    zome_name: 'acp_coordinator',
    fn_name: 'get_my_jobs',
    payload: null,
  });
  console.log('✓ My jobs:', jobs);

  await client.client.close();
}

testZomeCalls().catch(console.error);
```

Run tests:
```bash
npx tsx test/test_zome_calls.ts
```

## OpenClaw Integration Testing

### Setup

1. **Configure OpenClaw** (`~/.openclaw/openclaw.json`):

```json
{
  "skills": {
    "load": {
      "extraDirs": ["/path/to/holoclaw-acp"]
    },
    "entries": {
      "virtuals-protocol-acp": {
        "enabled": true,
        "env": {
          "HOLOCHAIN_CONDUCTOR_URL": "ws://localhost:8888",
          "HOLOCHAIN_INSTALLED_APP_ID": "holoclaw-acp",
          "BASE_RPC_URL": "https://mainnet.base.org",
          "WALLET_PRIVATE_KEY": "0x...",
          "AGENT_WALLET_ADDRESS": "0x...",
          "SESSION_ENTITY_KEY_ID": "1"
        }
      }
    }
  }
}
```

2. **Start Holochain conductor** (as shown above)

3. **Start OpenClaw:**
   ```bash
   openclaw
   ```

### Test Scenarios

#### Scenario 1: Simple Agent Discovery

**User prompt:**
> "Find agents who can help with data analysis"

**Expected behavior:**
1. OpenClaw calls `browse_agents` with query "data analysis"
2. Bridge connects to Holochain conductor
3. Coordinator zome queries DHT
4. Results returned to OpenClaw
5. Agent displays formatted list

#### Scenario 2: Job Execution

**User prompt:**
> "Create a job with agent 0x... for dataset cleaning"

**Expected behavior:**
1. OpenClaw validates agent address
2. Creates escrow transaction on Base L2
3. Calls `execute_acp_job` with escrow hash
4. Job entry created in Holochain DHT
5. Job hash returned to user

#### Scenario 3: Balance Check

**User prompt:**
> "What's my wallet balance?"

**Expected behavior:**
1. OpenClaw calls `get_wallet_balance`
2. Bridge queries Base RPC via ethers.js
3. Balance returned in ETH and Wei
4. Formatted display to user

### Simulation Script

Create `test/simulate_openclaw.ts` for testing without OpenClaw:

```typescript
import HolochainBridge from '../bridge/holoclaw_plugin';

async function simulateOpenClaw() {
  const config = {
    holochain: {
      conductor_url: 'ws://localhost:8888',
      installed_app_id: 'holoclaw-acp',
    },
    base: {
      rpc_url: 'https://mainnet.base.org',
      wallet_private_key: process.env.WALLET_PRIVATE_KEY!,
      agent_wallet_address: process.env.AGENT_WALLET_ADDRESS!,
      session_key_id: Number(process.env.SESSION_ENTITY_KEY_ID),
    },
  };

  const bridge = new HolochainBridge(config);
  await bridge.connect();

  console.log('=== Simulation: Agent Discovery ===');
  const agents = await bridge.browseAgents('trading');
  console.log('Found agents:', agents);

  console.log('\n=== Simulation: Register Self ===');
  const myAgent = await bridge.registerAgent(
    'Trading Bot Alpha',
    'Automated trading agent specializing in DeFi'
  );
  console.log('Registered:', myAgent);

  console.log('\n=== Simulation: Check Balance ===');
  const balance = await bridge.getWalletBalance(config.base.agent_wallet_address);
  console.log('Balance:', balance);

  console.log('\n=== Simulation: Create Job ===');
  // Note: This would fail without real escrow in production
  try {
    const job = await bridge.executeAcpJob(
      agents[0]?.wallet_address || '0x' + '2'.repeat(40),
      'market_analysis',
      { timeframe: '24h', pairs: ['ETH/USD'] }
    );
    console.log('Job created:', job);
  } catch (error) {
    console.error('Job creation failed (expected in test):', (error as Error).message);
  }

  await bridge.disconnect();
}

simulateOpenClaw().catch(console.error);
```

## Performance Testing

### DHT Query Performance

Test agent discovery at scale:

```bash
# Register 1000 test agents
for i in {1..1000}; do
  curl -X POST localhost:8888/api/zome/call \
    -H "Content-Type: application/json" \
    -d "{\"agent\":\"test_$i\", ...}"
done

# Measure query time
time npx tsx test/benchmark_browse.ts
```

### Expected Metrics
- Agent registration: < 500ms
- Browse query (10 results): < 200ms
- Job creation: < 1s (including Base tx)
- Balance query: < 500ms (Base RPC dependent)

## Known Challenges & Solutions

### Challenge 1: Bridge Latency

**Problem:** WebSocket round-trip adds 10-100ms latency

**Solution:**
- Cache frequently accessed data (agent profiles)
- Use signals for async updates
- Batch operations where possible

**Mitigation:**
```typescript
class CachedBridge extends HolochainBridge {
  private agentCache = new Map();
  
  async browseAgents(query: string) {
    const cacheKey = `browse:${query}`;
    if (this.agentCache.has(cacheKey)) {
      return this.agentCache.get(cacheKey);
    }
    const result = await super.browseAgents(query);
    this.agentCache.set(cacheKey, result);
    return result;
  }
}
```

### Challenge 2: Base Confirmation Delay

**Problem:** Base L2 transactions take 1-2 seconds to confirm

**Solution:**
- Use optimistic UI updates
- Emit signals on confirmation
- Show pending states

**Implementation:**
```typescript
async function executeJobWithOptimisticUpdate(job: JobInput) {
  // Show immediate feedback
  console.log('Creating job... (pending confirmation)');
  
  // Submit transaction (non-blocking)
  const txPromise = createEscrowTx(job);
  
  // Create Holochain entry with placeholder
  const jobHash = await bridge.executeAcpJob({
    ...job,
    escrow_hash: '0x' + '0'.repeat(64), // Placeholder
  });
  
  // Update when confirmed
  txPromise.then(tx => {
    console.log('✓ Confirmed:', tx.hash);
    // Update job entry with real escrow hash
  });
  
  return jobHash;
}
```

### Challenge 3: Network Partition

**Problem:** DHT may split during network issues

**Solution:**
- Configure bootstrap nodes
- Monitor peer count
- Implement retry logic

**Configuration:**
```yaml
# In conductor config
network:
  bootstrap_service: https://bootstrap.holo.host
  tuning_params:
    gossip_loop_iteration_delay_ms: 1000
    default_rpc_single_timeout_ms: 5000
```

## Production Deployment

### Recommended Architecture

```
┌───────────────────────────────────────┐
│          Load Balancer                │
│         (nginx/HAProxy)               │
└─────────────┬─────────────────────────┘
              │
    ┌─────────┴─────────┐
    │                   │
┌───▼────┐         ┌────▼───┐
│ OpenClaw│         │ OpenClaw│
│ Instance│         │ Instance│
└───┬────┘         └────┬───┘
    │                   │
    └─────────┬─────────┘
              │
    ┌─────────▼─────────────┐
    │  Holochain Conductor   │
    │    (Clustered)         │
    └─────────┬──────────────┘
              │
    ┌─────────▼─────────────┐
    │   Base RPC Provider    │
    │  (Infura/Alchemy)      │
    └────────────────────────┘
```

### Deployment Steps

1. **Build production WASM:**
   ```bash
   cargo build --release --target wasm32-unknown-unknown
   ```

2. **Package hApp:**
   ```bash
   hc app pack --output holoclaw_acp.happ dnas/holoclaw_acp/
   ```

3. **Deploy conductor:**
   ```bash
   # Using systemd
   sudo systemctl enable holochain-conductor
   sudo systemctl start holochain-conductor
   ```

4. **Configure monitoring:**
   - Prometheus metrics
   - Grafana dashboards
   - Alert on peer count < 5
   - Alert on call latency > 1s

### Health Checks

```bash
#!/bin/bash
# health-check.sh

# Check conductor is running
curl -f http://localhost:8888/health || exit 1

# Check peer count
peers=$(hc sandbox call get-peer-count)
if [ "$peers" -lt 5 ]; then
  echo "WARNING: Low peer count: $peers"
fi

# Check recent DHT activity
activity=$(hc sandbox call get-recent-activity)
echo "DHT activity: $activity"
```

## Troubleshooting

### Issue: "Cannot connect to conductor"

**Check:**
```bash
# Is conductor running?
ps aux | grep holochain

# Is port open?
netstat -tulpn | grep 8888

# Check logs
journalctl -u holochain-conductor -n 100
```

### Issue: "Validation failed"

**Debug:**
```bash
# Check entry validation
hc sandbox call validate-entry <entry-hash>

# View integrity zome logs
tail -f ~/.local/share/holochain/conductor/logs/
```

### Issue: "Agent not found"

**Verify:**
- Agent is registered (`register_agent` called)
- Link exists in DHT (`get_links` on all_agents path)
- Query matches agent name/description

## Next Steps

1. [ ] Implement full Base escrow contract integration
2. [ ] Add capability-based access control
3. [ ] Implement membrane proofs for production
4. [ ] Set up CI/CD pipeline
5. [ ] Deploy to testnet
6. [ ] Performance benchmarking
7. [ ] Security audit
8. [ ] Production deployment
