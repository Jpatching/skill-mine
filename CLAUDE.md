# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SKILL is a Solana-based skill-based mining protocol, forked from ORE. The key differentiator is the hybrid skill system that combines RNG-based gameplay with skill multipliers earned through challenges.

## Build Commands

```bash
# Using Makefile (recommended)
make build      # Build all workspace packages
make test       # Run Solana BPF tests
make format     # Format all code
make lint       # Run clippy linter
make check      # Full check (format + lint + build)
make help       # Show all available commands

# Or directly with Cargo
cargo build -p skill-api -p skill-program -p skill-cli
cargo test-sbf
cargo fmt --all
cargo clippy --all-targets -- -D warnings

# Run CLI commands
KEYPAIR=~/.config/solana/id.json RPC=https://api.devnet.solana.com COMMAND=board cargo run -p skill-cli
```

## Architecture

### Workspace Structure
- **api/** (`skill-api`) - Program constants, errors, events, instructions, SDK, and state definitions
- **program/** (`skill-program`) - On-chain Solana program implementation
- **cli/** (`skill-cli`) - Command-line interface for admin/testing operations
- **app/** - Dioxus web application (has separate dependencies, not part of workspace)

### Core State (api/src/state/)
Program accounts with PDA derivation:
- `Board` - Current round number and timing (seed: `BOARD`)
- `Round` - Game state for a specific round (seed: `ROUND + id`)
- `Miner` - Individual miner's state (seed: `MINER + authority`)
- `Automation` - Automation configuration (seed: `AUTOMATION + authority`)
- `Stake` - User staking activity (seed: `STAKE + authority`)
- `Treasury` - Token minting/burning/escrow (seed: `TREASURY`)
- `Config` - Global program configuration (seed: `CONFIG`)
- `Mint` - SKILL token mint (seed: `MINT`) - Treasury is mint authority

### Instructions (program/src/)
Each instruction has its own module:
- **Initialization**: `initialize` - One-time setup of Board, Config, Treasury, and SKILL mint
- **Mining**: `deploy`, `checkpoint`, `reset`, `claim_ore`, `claim_sol`, `automate`, `reload_sol`
- **Staking**: `deposit`, `withdraw`, `claim_yield`
- **Admin**: `bury`, `buyback`, `wrap`, `set_admin`, `set_fee_collector`

### Game Mechanics
1. **Round System**: 150-slot rounds (~1 minute)
2. **Board**: 25 squares where players deploy SOL
3. **RNG**: Entropy protocol determines winning square at round end
4. **Rewards**: Winners share losing SOL + token emissions

### Key Dependencies
- **steel** - Solana program framework
- **entropy-api** - External randomness source
- **spl-token/spl-token-2022** - Token program interactions

## CLI Usage

```bash
# Available commands (set via COMMAND env var):
# init, board, config, treasury, miner, round, stake, deploy, claim, reset, checkpoint, automations
KEYPAIR=/path/to/keypair.json RPC=https://rpc-url COMMAND=board cargo run -p skill-cli
```

### Initialization (First-Time Setup)

Before any gameplay, initialize the protocol:

```bash
# Initialize with defaults (payer becomes admin and fee collector)
KEYPAIR=~/.config/solana/id.json \
RPC=https://api.devnet.solana.com \
COMMAND=init \
cargo run -p skill-cli

# Initialize with custom parameters
KEYPAIR=~/.config/solana/id.json \
RPC=https://api.devnet.solana.com \
COMMAND=init \
ADMIN=<pubkey> \
FEE_COLLECTOR=<pubkey> \
VAR_ADDRESS=<entropy-var-pubkey> \
cargo run -p skill-cli
```

This creates:
- Board PDA (round tracking)
- Config PDA (admin configuration)
- Treasury PDA (token escrow)
- SKILL token mint (with Treasury as mint authority)
- Treasury's associated token account

## Devnet Deployment (Current)

**Program ID:** `3vzFzHFytiu7zkctgwX2JJhXq3XdN8J7U2WFongrejoU`

**Initialized Accounts:**
| Account | Address |
|---------|---------|
| Board | `924DVhXS3hXKVoLcSd7Uhi2B4k7DjTWm7UYYbft4d5pq` |
| Config | `J1MkbQ4Yu4zHhcj3B34XHfcqufpBpyjQoAxYwy1KsAXj` |
| Treasury | `75mND1dHyZcXntj2m4iFdT9ZwwDTbFCMjDDNQdyz2t2c` |
| SKILL Mint | `BAeSqDykZ4SUrHChTFXnWV1vazWMMwi3iDMA5okhF8eB` |
| Treasury Tokens | `FyDJZfkXcL6LWfS8dZyvUQAUrTp44ewNYXA3R69bwR4q` |

**Admin:** `BYnoVgMLftH28ERdnrWjeGmZvQwAmDm9CqCPiGNRBTHu`

## Development Tools

- **`rustfmt.toml`** - Code formatting (max_width=100, module imports)
- **`Makefile`** - Common commands (`make build`, `make test`, `make lint`)
- **`.editorconfig`** - Cross-editor settings (4-space indent, UTF-8)

## Development Notes

- Program ID in `api/src/lib.rs`: `3vzFzHFytiu7zkctgwX2JJhXq3XdN8J7U2WFongrejoU`
- `MINT_ADDRESS` is now derived from PDA (seed: `MINT`) - fully independent from ORE
- `ADMIN_ADDRESS` in `api/src/consts.rs` controls who can call `initialize` and admin functions
- The app/ directory has its own complex dependency tree and is not part of the main workspace
- Internal naming still uses `OreInstruction`, `OreAccount` enums - will be renamed in v0.2

### Key Constants (api/src/consts.rs)
- `ADMIN_ADDRESS` - Only this keypair can initialize and call admin functions
- `MINT_ADDRESS` - SKILL token mint (derived PDA, created during `init`)
- `TOKEN_DECIMALS` - 11 (100 billion units per SKILL token)
- `MAX_SUPPLY` - 5,000,000 SKILL tokens

## Skill System (Planned for v0.2)

The hybrid skill system will add:
- `SubmitPrediction` instruction for guessing winning squares
- Skill score tracking in `Miner` state
- Skill multipliers applied during `checkpoint`
