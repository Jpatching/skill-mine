# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SKILL is a Solana-based skill-based mining protocol, forked from ORE. The key differentiator is the hybrid skill system that combines RNG-based gameplay with skill multipliers earned through challenges.

## Build Commands

```bash
# Build the core packages (api, program, cli)
cargo build -p skill-api -p skill-program -p skill-cli

# Build and run tests using Solana BPF toolchain
cargo test-sbf

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

### Instructions (program/src/)
Each instruction has its own module:
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
# board, config, treasury, miner, round, stake, deploy, claim, reset, checkpoint, automations
KEYPAIR=/path/to/keypair.json RPC=https://rpc-url COMMAND=board cargo run -p skill-cli
```

## Development Notes

- Program ID is placeholder in `api/src/lib.rs` - generate new keypair before mainnet
- Token addresses in `api/src/consts.rs` are placeholders
- The app/ directory has its own complex dependency tree and is not part of the main workspace
- Internal naming still uses `OreInstruction`, `OreAccount` enums - will be renamed in v0.2

## Skill System (Planned for v0.2)

The hybrid skill system will add:
- `SubmitPrediction` instruction for guessing winning squares
- Skill score tracking in `Miner` state
- Skill multipliers applied during `checkpoint`
