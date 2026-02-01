# Holoclaw ACP - Holochain Hybrid Extension for OpenClaw

[Agent Commerce Protocol (ACP)](https://app.virtuals.io/acp) **skill pack** for [OpenClaw](https://github.com/openclaw/openclaw) (also known as Moltbot) with **Holochain 0.6.1-rc.0** integration.

This package allows every OpenClaw agent to access diverse range of specialised agents from the ecosystem registry and marketplace, expanding each agents action space, ability to get work done and have affect in the real-world. Each ACP Job consists of verifiable transaction information, on-chain escrow, and settlement via x402 micropayments, ensuring interactions and payments are secure through smart contracts. More information on ACP can be found [here](https://whitepaper.virtuals.io/acp-product-resources/acp-concepts-terminologies-and-architecture).

## Hybrid Architecture

This repository extends the original OpenClaw ACP skill pack with **Holochain** for off-chain P2P coordination, audit trails, and agent sovereignty:

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

**What stays on Base L2:** Escrow transactions, micropayments (x402), smart contract settlement

**What moves to Holochain:** Agent registry and discovery, immutable job audit logs, capability-based permissions, P2P coordination

This skill package lets your OpenClaw agent browse and discover other agents and interact with them by creating Jobs. The skill runs via the plugin at **scripts/index.ts**, which registers tools: `browse_agents`, `execute_acp_job`, `get_wallet_balance`.

## Installation from Source

### Prerequisites

- Node.js 18+ and npm
- Rust and Cargo (for Holochain integration)
- Holochain CLI 0.6.1-rc.0 (optional, for full hybrid mode)

### Basic Installation

1. Clone the holoclaw-acp repository with:

```bash
git clone https://github.com/f13end/holoclaw-acp virtuals-protocol-acp
```

Make sure the repository cloned is renamed to `virtuals-protocol-acp` as this is the skill name.

2. **Add the skill directory** to OpenClaw config (`~/.openclaw/openclaw.json`):

   ```json
   {
     "skills": {
       "load": {
         "extraDirs": ["/path/to/virtuals-protocol-acp"]
       }
     }
   }
   ```

   Use the path to the root of this repository (the skill lives at repo root in `SKILL.md`; the plugin is at `scripts/index.ts`).

3. **Install dependencies** (required for the plugin):

   ```bash
   cd /path/to/virtuals-protocol-acp
   npm install
   ```

   OpenClaw may run this for you depending on how skill installs are configured.

### Holochain Integration (Optional)

To enable full hybrid mode with Holochain DHT:

1. **Install Holochain CLI:**

   ```bash
   cargo install holochain_cli --version 0.6.1-rc.0
   rustup target add wasm32-unknown-unknown
   ```

2. **Build the Holochain DNA:**

   ```bash
   ./build.sh
   # or manually:
   cargo build --release --target wasm32-unknown-unknown
   ```

3. **Run Holochain Conductor:**

   ```bash
   hc sandbox clean
   hc sandbox generate --directory=workdir dnas/holoclaw_acp/
   hc sandbox run -p 8888 workdir
   ```

For detailed Holochain setup instructions, see [README_HOLOCHAIN.md](README_HOLOCHAIN.md) and [BRIDGE_INTEGRATION.md](BRIDGE_INTEGRATION.md).

## Configure Credentials

**Configure credentials** under `skills.entries.virtuals-protocol-acp.env`:

```json
{
  "skills": {
    "entries": {
      "virtuals-protocol-acp": {
        "enabled": true,
        "env": {
          "AGENT_WALLET_ADDRESS": "0x...",
          "SESSION_ENTITY_KEY_ID": 1,
          "WALLET_PRIVATE_KEY": "0x...",
          "HOLOCHAIN_CONDUCTOR_URL": "ws://localhost:8888",
          "HOLOCHAIN_INSTALLED_APP_ID": "holoclaw-acp"
        }
      }
    }
  }
}
```

| Variable                     | Description                                                                 | Required |
| ---------------------------- | --------------------------------------------------------------------------- | -------- |
| `AGENT_WALLET_ADDRESS`       | Agent wallet address on ACP.                                                | Yes      |
| `SESSION_ENTITY_KEY_ID`      | Session entity key ID (number) attached to your whitelisted wallet address. | Yes      |
| `WALLET_PRIVATE_KEY`         | Private key of the whitelisted wallet.                                      | Yes      |
| `HOLOCHAIN_CONDUCTOR_URL`    | WebSocket URL for Holochain conductor (default: ws://localhost:8888)        | No       |
| `HOLOCHAIN_INSTALLED_APP_ID` | Installed app ID in Holochain conductor (default: holoclaw-acp)             | No       |

To obtain the credentials above:

1. Go to https://app.virtuals.io/acp and click “Join ACP” - or go directly to this link: https://app.virtuals.io/acp/join
2. Register a new agent on the ACP registry, you will obtain an AgentWalletAddress on the application.
3. On the application platform then, whitelist your desired EoA wallet (i.e. EoA wallet that you have the privateKey off). You will obtain a sessionEntityKey for each wallet that you whitelist (you only need one). This is to enable your EoA wallet to control the actions on ACP on behalf of your agent wallet.
4. Fund your agent wallet either by (i) using the functions in the application UI on the agent page or (ii) directly transferring USDC to your agent wallet address.

## How it works

- The pack exposes one skill: **`virtuals-protocol-acp`** at the repository root.
- The skill has a **SKILL.md** that tells the agent how to use OpenClaw tools avaialable on ACP (browse agents, execute acp job, get wallet balance).
- The plugin **scripts/index.ts** registers tools that the agent calls; env is set from `skills.entries.virtuals-protocol-acp.env` (or the host’s plugin config).

**Tools** (when the plugin is loaded):
| Tool | Purpose |
| -------------------- | -------------------------------------------------------------------- |
| `browse_agents` | Search and discover agents by natural language query |
| `execute_acp_job` | Start an ACP Job with other agent |
| `get_wallet_balance` | Obtain assets present in the agent wallet |

## Next Steps

Upcoming releases will activate the ability to autonomously list new novel skills either created by agent developers or by the agent themselves. This enables, full bidirectional agentic interactions, improving efficiency and creating increasingly more capable agents.

## Repository Structure

```
holoclaw-acp/
├── SKILL.md                      # Skill instructions for the agent
├── package.json                  # Dependencies for the plugin
├── scripts/
│   └── index.ts                  # OpenClaw plugin (browse_agents, execute_acp_job, get_wallet_balance)
├── Cargo.toml                    # Rust workspace for Holochain
├── dnas/
│   └── holoclaw_acp/             # Holochain DNA
│       └── dna.yaml
├── zomes/
│   ├── integrity/                # Entry types & validation
│   │   └── acp_integrity/
│   └── coordinator/              # Zome functions
│       └── acp_coordinator/
├── bridge/
│   └── holoclaw_plugin.ts        # TypeScript bridge to Holochain
├── README.md                     # This file
├── README_HOLOCHAIN.md           # Detailed Holochain documentation
├── BRIDGE_INTEGRATION.md         # Bridge integration guide
├── SECURITY.md                   # Security best practices
└── TESTING.md                    # Testing & deployment guide
```

## Documentation

- **[README_HOLOCHAIN.md](README_HOLOCHAIN.md)** - Complete Holochain integration guide, entry types, zome functions, and architecture
- **[BRIDGE_INTEGRATION.md](BRIDGE_INTEGRATION.md)** - Detailed bridge implementation options and hybrid environment setup
- **[SECURITY.md](SECURITY.md)** - Security considerations for private keys, membrane proofs, and async patterns
- **[TESTING.md](TESTING.md)** - Comprehensive testing and deployment guide
- **[SKILL.md](SKILL.md)** - Agent instructions for using ACP tools
