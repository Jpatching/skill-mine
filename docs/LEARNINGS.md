# SKILL-MINE Development Learnings

## ORE Architecture Insights

### Round-Based Game Mechanics
- Rounds last ~150 slots (~1 minute)
- 25 squares on a board, players deploy SOL to claim positions
- At round end, RNG selects winning square
- Winners on that square share the losing SOL + token emissions

### RNG Points in ORE
The following use randomness from Entropy protocol:
1. **Winning square selection**: `rng % 25`
2. **Top miner within square**: `rng.reverse_bits() % deployed[square]`
3. **Split reward probability**: 50% chance (XOR of 4 u16 chunks)
4. **Motherlode trigger**: 1/625 chance

### State Organization
- Separate accounts for: Board (current), Round (per-round), Miner (per-authority), Treasury (singleton)
- Enables parallel processing - miners' accounts are independent
- Round accounts expire and close, limiting historical data

### Position Tracking
Per-miner per-square tracking enables deterministic winner selection:
- `miner.deployed[square]` - Miner's SOL on that square
- `miner.cumulative[square]` - Total SOL on square BEFORE this miner's deployment
- Winner is selected based on where `rng_sample` falls in the cumulative ranges

## Solana Development Notes

### Steel Framework
- Provides account parsing macros and instruction routing
- Uses discriminator-based account identification
- Simplifies PDA derivation and validation
- `instruction!` macro auto-generates deserialization code

### BPF Build Requirements
- Use `cargo test-sbf` for full program tests
- Requires Solana toolchain installation
- Some dependencies need specific Rust versions (currently 1.83+)

### Common Patterns
- Use `sol_log` for on-chain debugging
- PDAs derived with consistent seeds across state modules
- Instruction data parsed via `steel::parse_instruction`
- Account size validated with `8 + std::mem::size_of::<T>()`

### Account Sizing
Current Miner account: 536 bytes (+ 8 byte discriminator = 544 total)
- Adding skill fields increases by ~44 bytes
- Must update size checks when extending accounts
- Rent costs scale linearly with account size

## v0.1 Implementation Learnings

### Initialize Instruction Pattern
```rust
// PDA creation with CPI signing
invoke_signed(
    &system_instruction::create_account(...),
    &[accounts...],
    &[&[SEED, &[bump]]],  // Signer seeds
)?;
```

### Key Insights from v0.1
1. **PDA-derived mint** - SKILL mint is a PDA, not external address
2. **Treasury as mint authority** - Enables controlled token minting
3. **Flexible entropy source** - var_address stored in Config, not hardcoded
4. **CLI env var pattern** - `COMMAND=init ADMIN=<pubkey>` pattern works well

## Security Considerations

### Challenge System Design
- Predictions must be submitted BEFORE round ends
- Verify `last_prediction_round < current_round` to prevent retroactive predictions
- On-chain timestamp verification via `Clock` sysvar

### Multiplier Caps
- Unbounded skill multipliers could be exploited
- Chosen formula: `base + log(score) * 5 + streak * 2`, capped at 150 (1.5x)
- Logarithmic scaling prevents runaway multipliers
- Streak capped at 10 for max +20% bonus

### Sybil Resistance
- Single account could be gamed with multiple wallets
- Potential mitigations:
  - Stake-weighted skill (higher stake = more skill points per correct prediction)
  - Cooldown periods between account creation and participation
  - Identity verification through staking history

### Front-Running
- Prediction submissions are visible in mempool
- Mitigation: Commit-reveal scheme (hash of prediction, then reveal after round)
- Simpler alternative: Short submission windows before round end

## Design Decisions

### Why Hybrid System?
- Pure RNG (like ORE) is gameable only by capital size
- Pure skill puzzles can be solved with better hardware/bots
- Hybrid gives skilled humans an edge without eliminating capital importance

### Why Prediction Challenges?
- Simple to implement and verify on-chain
- Doesn't favor hardware (all users can guess)
- Creates engagement and strategy beyond just deploying capital
- Streak bonuses reward consistent play over bot-like spray patterns

### Why Keep ORE Base?
- Proven tokenomics and game theory
- Battle-tested smart contract code
- Community familiarity with mechanics
- Reduces audit scope to skill additions only

## v0.2 Skill System Design

### Miner State Extensions
```rust
pub skill_score: u64,           // Cumulative points (never decreases)
pub prediction: u8,             // Current guess (0-24, 255=none)
pub streak: u16,                // Consecutive correct predictions
pub last_prediction_round: u64, // Anti-replay protection
pub challenge_count: u64,       // Total prediction attempts
pub challenge_wins: u64,        // Total correct predictions
```

### Checkpoint Integration
Skill multiplier applied after base reward calculation:
```rust
// After calculating base rewards_ore
let multiplier = miner.calculate_skill_multiplier();
let boosted = (rewards_ore * multiplier / 100).min(MAX_REWARD);
miner.rewards_ore = boosted;
```

### SubmitPrediction Flow
1. User calls SubmitPrediction with square (0-24)
2. Instruction validates:
   - Round is active (not ended)
   - User hasn't predicted this round yet
3. Stores prediction in miner.prediction
4. Updates last_prediction_round
5. At checkpoint:
   - Compare prediction to winning_square
   - Update skill_score, streak, challenge_count, challenge_wins
