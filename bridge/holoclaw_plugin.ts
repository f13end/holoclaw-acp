/**
 * Holochain-OpenClaw Bridge Plugin
 * 
 * This TypeScript plugin integrates Holochain DHT functionality with OpenClaw's
 * existing ACP skill pack. It provides a hybrid approach where:
 * - Holochain handles agent discovery, job provenance, and P2P coordination
 * - Base L2 handles escrow, payments, and settlement
 * 
 * Usage: Import this in OpenClaw instead of scripts/index.ts
 */

import { AppWebsocket, CellId, InstalledAppInfo } from '@holochain/client';
import { ethers } from 'ethers';

interface HolochainConfig {
  conductor_url: string;
  installed_app_id: string;
}

interface BaseConfig {
  rpc_url: string;
  wallet_private_key: string;
  agent_wallet_address: string;
  session_key_id: number;
}

interface HybridConfig {
  holochain: HolochainConfig;
  base: BaseConfig;
}

/**
 * Holochain Bridge Client
 * Manages connection to Holochain conductor and provides zome call wrappers
 */
export class HolochainBridge {
  private client: AppWebsocket | null = null;
  private appInfo: InstalledAppInfo | null = null;
  private cellId: CellId | null = null;

  constructor(private config: HybridConfig) {}

  async connect(): Promise<void> {
    this.client = await AppWebsocket.connect(this.config.holochain.conductor_url);
    this.appInfo = await this.client.appInfo({
      installed_app_id: this.config.holochain.installed_app_id,
    });
    
    if (!this.appInfo || this.appInfo.cell_data.length === 0) {
      throw new Error('No cells found in installed app');
    }

    this.cellId = this.appInfo.cell_data[0].cell_id;
  }

  private async callZome(zomeName: string, fnName: string, payload: any): Promise<any> {
    if (!this.client || !this.cellId) {
      throw new Error('Not connected to Holochain conductor');
    }

    return await this.client.callZome({
      cap_secret: null,
      cell_id: this.cellId,
      zome_name: zomeName,
      fn_name: fnName,
      payload,
    });
  }

  /**
   * Browse agents from Holochain DHT
   */
  async browseAgents(query: string): Promise<any[]> {
    return await this.callZome('acp_coordinator', 'browse_agents', {
      query,
    });
  }

  /**
   * Execute ACP job with Holochain provenance
   * 
   * Steps:
   * 1. Create escrow transaction on Base L2
   * 2. Record job in Holochain for immutable provenance
   * 3. Return job hash for tracking
   */
  async executeAcpJob(
    agentWalletAddress: string,
    jobOfferingName: string,
    serviceRequirements: Record<string, any>
  ): Promise<any> {
    // Step 1: Create escrow transaction on Base
    const escrowHash = await this.createEscrowTransaction(
      agentWalletAddress,
      serviceRequirements
    );

    // Step 2: Record job in Holochain DHT
    const jobHash = await this.callZome('acp_coordinator', 'execute_acp_job', {
      agent_wallet_address: agentWalletAddress,
      job_offering_name: jobOfferingName,
      service_requirements: serviceRequirements,
      escrow_hash: escrowHash,
    });

    return {
      job_hash: jobHash,
      escrow_hash: escrowHash,
      status: 'created',
    };
  }

  /**
   * Get wallet balance from Base L2
   */
  async getWalletBalance(address: string): Promise<any> {
    const provider = new ethers.JsonRpcProvider(this.config.base.rpc_url);
    const balance = await provider.getBalance(address);

    return {
      address,
      balance_wei: balance.toString(),
      balance_eth: ethers.formatEther(balance),
    };
  }

  /**
   * Register agent in Holochain DHT
   */
  async registerAgent(name: string, description: string): Promise<any> {
    const now = Math.floor(Date.now() / 1000);
    
    return await this.callZome('acp_coordinator', 'register_agent', {
      wallet_address: this.config.base.agent_wallet_address,
      session_key_id: this.config.base.session_key_id,
      name,
      description,
      registered_at: now,
    });
  }

  /**
   * Get all jobs for the current agent
   */
  async getMyJobs(): Promise<any[]> {
    return await this.callZome('acp_coordinator', 'get_my_jobs', null);
  }

  /**
   * Create escrow transaction on Base L2
   * 
   * In production, this would:
   * 1. Estimate gas
   * 2. Create and sign transaction
   * 3. Submit to Base network
   * 4. Wait for confirmation
   * 5. Return transaction hash
   */
  private async createEscrowTransaction(
    recipientAddress: string,
    requirements: Record<string, any>
  ): Promise<string> {
    // Placeholder implementation
    // In production, integrate with Base escrow contract
    
    const provider = new ethers.JsonRpcProvider(this.config.base.rpc_url);
    const wallet = new ethers.Wallet(this.config.base.wallet_private_key, provider);

    // Example: Simple ETH transfer (replace with escrow contract call)
    // const tx = await wallet.sendTransaction({
    //   to: escrowContractAddress,
    //   value: ethers.parseEther('0.01'),
    //   data: encodedEscrowData,
    // });
    // await tx.wait();
    // return tx.hash;

    // For now, return a mock transaction hash
    const mockHash = '0x' + Buffer.from(
      `escrow_${Date.now()}_${recipientAddress}`
    ).toString('hex').slice(0, 64).padEnd(64, '0');
    
    return mockHash;
  }

  async disconnect(): Promise<void> {
    if (this.client) {
      await this.client.client.close();
      this.client = null;
    }
  }
}

/**
 * CLI wrapper for OpenClaw compatibility
 * 
 * This maintains compatibility with the existing scripts/index.ts interface
 * while adding Holochain functionality.
 */
export async function runHybridCli(): Promise<void> {
  const config: HybridConfig = {
    holochain: {
      conductor_url: process.env.HOLOCHAIN_CONDUCTOR_URL || 'ws://localhost:8888',
      installed_app_id: process.env.HOLOCHAIN_INSTALLED_APP_ID || 'holoclaw-acp',
    },
    base: {
      rpc_url: process.env.BASE_RPC_URL || 'https://mainnet.base.org',
      wallet_private_key: process.env.WALLET_PRIVATE_KEY!,
      agent_wallet_address: process.env.AGENT_WALLET_ADDRESS!,
      session_key_id: Number(process.env.SESSION_ENTITY_KEY_ID),
    },
  };

  const bridge = new HolochainBridge(config);
  await bridge.connect();

  const [, , command, ...args] = process.argv;

  try {
    let result: any;

    switch (command) {
      case 'browse_agents':
        result = await bridge.browseAgents(args[0] || '');
        break;

      case 'execute_acp_job':
        const serviceReqs = args[2] ? JSON.parse(args[2]) : {};
        result = await bridge.executeAcpJob(args[0]!, args[1]!, serviceReqs);
        break;

      case 'get_wallet_balance':
        const address = args[0] || config.base.agent_wallet_address;
        result = await bridge.getWalletBalance(address);
        break;

      case 'register_agent':
        result = await bridge.registerAgent(args[0]!, args[1] || '');
        break;

      case 'get_my_jobs':
        result = await bridge.getMyJobs();
        break;

      default:
        throw new Error(
          'Usage: browse_agents <query> | execute_acp_job <addr> <job> <reqs> | get_wallet_balance [addr] | register_agent <name> <desc> | get_my_jobs'
        );
    }

    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(JSON.stringify({ error: (error as Error).message }));
    process.exit(1);
  } finally {
    await bridge.disconnect();
  }
}

// Export for use in other modules
export default HolochainBridge;
