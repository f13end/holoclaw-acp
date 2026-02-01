use hdk::prelude::*;
use acp_integrity::{AcpAgent, AcpJob, EntryTypes, LinkTypes};
use std::collections::HashMap;

/// Query input for browsing agents
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BrowseAgentsQuery {
    pub query: String,
}

/// Response for browse_agents function
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AcpAgentInfo {
    pub wallet_address: String,
    pub session_key_id: u64,
    pub name: String,
    pub description: String,
    pub registered_at: u64,
    pub agent_hash: ActionHash,
}

/// Job creation input
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobCreationInput {
    pub agent_wallet_address: String,
    pub job_offering_name: String,
    pub service_requirements: HashMap<String, String>,
    pub escrow_hash: String,
}

/// Balance query response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletBalance {
    pub address: String,
    pub balance_wei: String,
    pub balance_eth: String,
}

// ============================================================================
// ANCHOR PATHS
// ============================================================================

/// Get the path for all agents anchor
fn all_agents_path() -> ExternResult<Path> {
    Path::from("all_agents").typed(LinkTypes::AllAgents)
}

// ============================================================================
// ZOME FUNCTIONS
// ============================================================================

/// Register a new ACP Agent in the DHT
#[hdk_extern]
pub fn register_agent(agent: AcpAgent) -> ExternResult<ActionHash> {
    // Create the agent entry
    let agent_hash = create_entry(EntryTypes::AcpAgent(agent.clone()))?;

    // Link from agent's pubkey to their profile
    let my_pub_key = agent_info()?.agent_initial_pubkey;
    create_link(
        my_pub_key.clone(),
        agent_hash.clone(),
        LinkTypes::AgentToProfile,
        (),
    )?;

    // Link from all_agents anchor for discovery
    let all_agents_path = all_agents_path()?;
    all_agents_path.ensure()?;
    create_link(
        all_agents_path.path_entry_hash()?,
        agent_hash.clone(),
        LinkTypes::AllAgents,
        (),
    )?;

    Ok(agent_hash)
}

/// Browse agents using DHT query with basic filtering
/// 
/// This function searches the DHT for registered agents and filters by query string.
/// In production, you would integrate more sophisticated search/indexing.
#[hdk_extern]
pub fn browse_agents(input: BrowseAgentsQuery) -> ExternResult<Vec<AcpAgentInfo>> {
    let all_agents_path = all_agents_path()?;
    
    // Get all links from the all_agents anchor
    let links = get_links(
        GetLinksInputBuilder::try_new(
            all_agents_path.path_entry_hash()?,
            LinkTypes::AllAgents,
        )?
        .build(),
    )?;

    let mut agents: Vec<AcpAgentInfo> = Vec::new();

    for link in links {
        // Get the agent entry
        if let Some(record) = get(link.target.clone(), GetOptions::default())? {
            if let Some(entry) = record.entry().as_option() {
                if let Some(agent) = entry.clone().into_app_data::<AcpAgent>().ok() {
                    // Basic filtering by query (case-insensitive search in name/description)
                    let query_lower = input.query.to_lowercase();
                    let matches_query = agent.name.to_lowercase().contains(&query_lower)
                        || agent.description.to_lowercase().contains(&query_lower);

                    if input.query.is_empty() || matches_query {
                        agents.push(AcpAgentInfo {
                            wallet_address: agent.wallet_address,
                            session_key_id: agent.session_key_id,
                            name: agent.name,
                            description: agent.description,
                            registered_at: agent.registered_at,
                            agent_hash: record.action_address().clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(agents)
}

/// Execute an ACP job - creates immutable job entry and bridges to Base for escrow
/// 
/// This function:
/// 1. Creates an AcpJob entry in the source chain (immutable provenance)
/// 2. Links the job to the agent for job history
/// 3. Bridges to Base L2 to verify escrow transaction
/// 
/// Note: The actual escrow transaction should be created by the caller before calling this.
/// This function validates the escrow_hash format and stores the job provenance.
#[hdk_extern]
pub fn execute_acp_job(input: JobCreationInput) -> ExternResult<ActionHash> {
    // Get current timestamp
    let now = sys_time()?.as_micros();

    // Create job entry
    let job = AcpJob {
        job_id: format!("job_{}", now), // Generate unique job ID
        phases: vec![
            "requested".to_string(),
            "negotiation".to_string(),
            "transaction".to_string(),
        ],
        escrow_hash: input.escrow_hash.clone(),
        agent_wallet_address: input.agent_wallet_address,
        job_offering_name: input.job_offering_name,
        service_requirements: serde_json::to_string(&input.service_requirements)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?,
        created_at: now,
        current_phase: "requested".to_string(),
        deliverable: None,
    };

    // Create the job entry (will be validated by integrity zome)
    let job_hash = create_entry(EntryTypes::AcpJob(job.clone()))?;

    // Link job to agent's pubkey for job history
    let my_pub_key = agent_info()?.agent_initial_pubkey;
    create_link(
        my_pub_key,
        job_hash.clone(),
        LinkTypes::AgentToJobs,
        (),
    )?;

    // NOTE: In production, here you would:
    // 1. Call external function to verify escrow tx on Base L2
    // 2. Use ethers.rs to query the transaction status
    // 3. Emit a signal when job status changes
    // 
    // For now, we return the job hash as proof of job creation
    // The actual Base integration requires external calls which need
    // to be handled through capability grants or external service

    Ok(job_hash)
}

/// Get wallet balance from Base L2
/// 
/// This function queries the EVM balance via an external RPC call.
/// In production, this would use ethers.rs with an HTTP provider.
/// 
/// Note: Since HDK doesn't support direct HTTP calls, this needs to be
/// handled through:
/// 1. External service bridge (recommended)
/// 2. Capability grants to external binary
/// 3. Signal-based async pattern
#[hdk_extern]
pub fn get_wallet_balance(address: String) -> ExternResult<WalletBalance> {
    // Validate address format
    if !address.starts_with("0x") || address.len() != 42 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Invalid Ethereum address format".to_string()
        )));
    }

    // NOTE: Actual balance query needs to be done via external call
    // This is a placeholder that would be replaced with:
    // - Call to external service via capability grant
    // - Bridge to Node.js service that runs ethers.js
    // - WebSocket message to OpenClaw runtime
    //
    // For now, return a placeholder response
    // In production, integrate with Base RPC using external service

    Ok(WalletBalance {
        address: address.clone(),
        balance_wei: "0".to_string(), // Placeholder - integrate with Base RPC
        balance_eth: "0.0".to_string(),
    })
}

/// Get all jobs for the current agent
#[hdk_extern]
pub fn get_my_jobs(_: ()) -> ExternResult<Vec<AcpJob>> {
    let my_pub_key = agent_info()?.agent_initial_pubkey;
    
    let links = get_links(
        GetLinksInputBuilder::try_new(my_pub_key, LinkTypes::AgentToJobs)?.build(),
    )?;

    let mut jobs = Vec::new();
    for link in links {
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(entry) = record.entry().as_option() {
                if let Ok(job) = entry.clone().into_app_data::<AcpJob>() {
                    jobs.push(job);
                }
            }
        }
    }

    Ok(jobs)
}

/// Get a specific job by its ActionHash
#[hdk_extern]
pub fn get_job(job_hash: ActionHash) -> ExternResult<Option<AcpJob>> {
    if let Some(record) = get(job_hash, GetOptions::default())? {
        if let Some(entry) = record.entry().as_option() {
            if let Ok(job) = entry.clone().into_app_data::<AcpJob>() {
                return Ok(Some(job));
            }
        }
    }
    Ok(None)
}
