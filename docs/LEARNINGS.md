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

### BPF Build Requirements
- Use `cargo test-sbf` for full program tests
- Requires Solana toolchain installation
- Some dependencies need specific Rust versions (currently 1.83+)

### Common Patterns
- Use `sol_log` for on-chain debugging
- PDAs derived with consistent seeds across state modules
- Instruction data parsed via `steel::parse_instruction`

## Security Considerations

### Challenge System Design
- Predictions must be submitted BEFORE round ends
- Verify `last_challenge_round < current_round` to prevent retroactive predictions
- On-chain timestamp verification via `Clock` sysvar

### Multiplier Caps
- Unbounded skill multipliers could be exploited
- Consider logarithmic scaling: `1.0 + log(skill_score) / DIVISOR`
- Or hard caps: `min(skill_multiplier, MAX_MULTIPLIER)`

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
