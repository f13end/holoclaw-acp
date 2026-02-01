/**
 * Example test for Holochain zome calls
 * 
 * Prerequisites:
 * 1. Build DNA: ./build.sh
 * 2. Run conductor: hc sandbox run -p 8888 workdir
 * 3. Run this test: npx tsx test/test_zome_calls.ts
 */

import { AppWebsocket } from '@holochain/client';

async function testZomeCalls() {
  console.log('Connecting to Holochain conductor...');
  
  try {
    const client = await AppWebsocket.connect('ws://localhost:8888');
    console.log('✓ Connected');
    
    // Get app info
    const apps = await client.appInfo();
    console.log('Available apps:', apps);
    
    // Note: You need to install the app first using:
    // hc sandbox call install-app holoclaw_acp.happ
    
    const appId = process.env.HOLOCHAIN_INSTALLED_APP_ID || 'holoclaw-acp';
    const appInfo = await client.appInfo({ installed_app_id: appId });
    
    if (!appInfo || appInfo.cell_data.length === 0) {
      console.error('App not installed. Install with: hc sandbox install-app');
      process.exit(1);
    }
    
    const cellId = appInfo.cell_data[0].cell_id;
    console.log('Cell ID:', cellId);
    
    // Test 1: Register agent
    console.log('\n=== Test 1: Register Agent ===');
    const agentHash = await client.callZome({
      cap_secret: null,
      cell_id: cellId,
      zome_name: 'acp_coordinator',
      fn_name: 'register_agent',
      payload: {
        wallet_address: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1',
        session_key_id: 1,
        name: 'Test Trading Agent',
        description: 'Automated trading agent specializing in DeFi strategies',
        registered_at: Math.floor(Date.now() / 1000),
      },
    });
    console.log('✓ Agent registered:', agentHash);
    
    // Test 2: Browse agents
    console.log('\n=== Test 2: Browse Agents ===');
    const agents = await client.callZome({
      cap_secret: null,
      cell_id: cellId,
      zome_name: 'acp_coordinator',
      fn_name: 'browse_agents',
      payload: { query: '' },
    });
    console.log('✓ Found agents:', JSON.stringify(agents, null, 2));
    
    // Test 3: Browse with filter
    console.log('\n=== Test 3: Browse with Filter (trading) ===');
    const tradingAgents = await client.callZome({
      cap_secret: null,
      cell_id: cellId,
      zome_name: 'acp_coordinator',
      fn_name: 'browse_agents',
      payload: { query: 'trading' },
    });
    console.log('✓ Found trading agents:', JSON.stringify(tradingAgents, null, 2));
    
    // Test 4: Execute job
    console.log('\n=== Test 4: Execute ACP Job ===');
    const mockEscrowHash = '0x' + '1'.repeat(64);
    const jobHash = await client.callZome({
      cap_secret: null,
      cell_id: cellId,
      zome_name: 'acp_coordinator',
      fn_name: 'execute_acp_job',
      payload: {
        agent_wallet_address: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1',
        job_offering_name: 'market_analysis',
        service_requirements: {
          timeframe: '24h',
          pairs: ['ETH/USD', 'BTC/USD'],
        },
        escrow_hash: mockEscrowHash,
      },
    });
    console.log('✓ Job created:', jobHash);
    
    // Test 5: Get my jobs
    console.log('\n=== Test 5: Get My Jobs ===');
    const jobs = await client.callZome({
      cap_secret: null,
      cell_id: cellId,
      zome_name: 'acp_coordinator',
      fn_name: 'get_my_jobs',
      payload: null,
    });
    console.log('✓ My jobs:', JSON.stringify(jobs, null, 2));
    
    // Test 6: Get wallet balance (will use placeholder)
    console.log('\n=== Test 6: Get Wallet Balance ===');
    const balance = await client.callZome({
      cap_secret: null,
      cell_id: cellId,
      zome_name: 'acp_coordinator',
      fn_name: 'get_wallet_balance',
      payload: '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1',
    });
    console.log('✓ Balance:', JSON.stringify(balance, null, 2));
    
    console.log('\n=== All Tests Passed ===');
    
    await client.client.close();
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

testZomeCalls().catch(console.error);
