use hdi::prelude::*;

/// ACP Agent entry - represents an agent registered in the DHT
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AcpAgent {
    /// Base L2 wallet address (0x...)
    pub wallet_address: String,
    /// Session entity key ID for wallet authentication
    pub session_key_id: u64,
    /// Agent name for discovery
    pub name: String,
    /// Agent description
    pub description: String,
    /// Registration timestamp
    pub registered_at: u64,
}

/// ACP Job entry - immutable provenance record for job execution
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AcpJob {
    /// Unique job identifier
    pub job_id: String,
    /// Job phases: requested, negotiation, transaction, completed, rejected
    pub phases: Vec<String>,
    /// Base L2 escrow transaction hash
    pub escrow_hash: String,
    /// Target agent wallet address
    pub agent_wallet_address: String,
    /// Job offering name
    pub job_offering_name: String,
    /// Service requirements (JSON string)
    pub service_requirements: String,
    /// Job creation timestamp
    pub created_at: u64,
    /// Current phase
    pub current_phase: String,
    /// Deliverable (if completed)
    pub deliverable: Option<String>,
}

#[hdk_entry_defs]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    AcpAgent(AcpAgent),
    AcpJob(AcpJob),
}

#[hdk_link_types]
pub enum LinkTypes {
    AgentToProfile,
    AllAgents,
    AgentToJobs,
}

/// Validate AcpAgent entry
pub fn validate_acp_agent(agent: AcpAgent) -> ExternResult<ValidateCallbackResult> {
    // Validate wallet address format (basic 0x... check)
    if !agent.wallet_address.starts_with("0x") || agent.wallet_address.len() != 42 {
        return Ok(ValidateCallbackResult::Invalid(
            "wallet_address must be a valid Ethereum address (0x... 42 chars)".to_string(),
        ));
    }

    // Validate name is not empty
    if agent.name.trim().is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "name cannot be empty".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate AcpJob entry
pub fn validate_acp_job(job: AcpJob) -> ExternResult<ValidateCallbackResult> {
    // Validate escrow_hash format (must be valid Base tx hash format)
    if !job.escrow_hash.starts_with("0x") || job.escrow_hash.len() != 66 {
        return Ok(ValidateCallbackResult::Invalid(
            "escrow_hash must be a valid transaction hash (0x... 66 chars)".to_string(),
        ));
    }

    // Validate agent_wallet_address format
    if !job.agent_wallet_address.starts_with("0x")
        || job.agent_wallet_address.len() != 42
    {
        return Ok(ValidateCallbackResult::Invalid(
            "agent_wallet_address must be a valid Ethereum address".to_string(),
        ));
    }

    // Validate phases is not empty
    if job.phases.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "phases cannot be empty".to_string(),
        ));
    }

    // Validate job_id is not empty
    if job.job_id.trim().is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "job_id cannot be empty".to_string(),
        ));
    }

    // Validate current_phase is one of the expected values
    let valid_phases = vec!["requested", "negotiation", "transaction", "completed", "rejected"];
    if !valid_phases.contains(&job.current_phase.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("current_phase must be one of: {}", valid_phases.join(", ")),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op {
        Op::StoreRecord(store_record) => {
            match store_record.record.entry().as_option() {
                Some(entry) => {
                    // Try to deserialize as AcpAgent
                    if let Ok(agent) = AcpAgent::try_from(entry) {
                        return validate_acp_agent(agent);
                    }
                    // Try to deserialize as AcpJob
                    if let Ok(job) = AcpJob::try_from(entry) {
                        return validate_acp_job(job);
                    }
                    Ok(ValidateCallbackResult::Valid)
                }
                None => Ok(ValidateCallbackResult::Valid),
            }
        }
        Op::RegisterCreateLink(_create_link) => {
            // Validate link creation - all link types allowed for now
            Ok(ValidateCallbackResult::Valid)
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
