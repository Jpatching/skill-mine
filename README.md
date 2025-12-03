# SKILL

SKILL is a skill-based mining protocol on Solana, forked from ORE.

## Overview

SKILL combines ORE's proven round-based mining mechanics with a skill layer that rewards strategic players. While the base game uses RNG to determine winners, skilled players can earn multipliers through prediction challenges and strategic play.

## Key Features

- **Round-Based Mining**: Deploy SOL to 25 squares on a board each round
- **Skill Multipliers**: Earn bonus rewards through correct predictions
- **Streak Bonuses**: Consistent correct predictions increase your multiplier
- **Fair Play**: Skill challenges are designed to resist botting and hardware advantages

## Architecture

### API
- [`Consts`](api/src/consts.rs) – Program constants
- [`Error`](api/src/error.rs) – Custom program errors
- [`Event`](api/src/event.rs) – Custom program events
- [`Instruction`](api/src/instruction.rs) – Declared instructions and arguments

### Instructions

#### Mining
- [`Deploy`](program/src/deploy.rs) – Deploy SOL to claim space on the board
- [`Checkpoint`](program/src/checkpoint.rs) – Checkpoint rewards from a prior round
- [`ClaimORE`](program/src/claim_ore.rs) – Claim token mining rewards
- [`ClaimSOL`](program/src/claim_sol.rs) – Claim SOL mining rewards
- [`Reset`](program/src/reset.rs) – Reset the board for a new round

#### Staking
- [`Deposit`](program/src/deposit.rs) – Deposit tokens into a stake account
- [`Withdraw`](program/src/withdraw.rs) – Withdraw tokens from a stake account
- [`ClaimYield`](program/src/claim_yield.rs) – Claim staking yield

### State
- [`Board`](api/src/state/board.rs) – Current round number and timestamps
- [`Round`](api/src/state/round.rs) – Game state of a given round
- [`Miner`](api/src/state/miner.rs) – Tracks a miner's game state
- [`Treasury`](api/src/state/treasury.rs) – Mints, burns, and escrows tokens
- [`Stake`](api/src/state/stake.rs) – Manages a user's staking activity

## Development

### Build

```bash
cargo build -p skill-api -p skill-program -p skill-cli
```

### Test

```bash
cargo test-sbf
```

### CLI

```bash
KEYPAIR=~/.config/solana/id.json RPC=https://api.devnet.solana.com COMMAND=board cargo run -p skill-cli
```

## License

Apache-2.0
