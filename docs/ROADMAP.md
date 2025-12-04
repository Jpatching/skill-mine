# SKILL-MINE Roadmap

## v0.1 - Clean Fork ✅ COMPLETE
- [x] Clone fresh ORE repositories (program, CLI, app)
- [x] Rebrand package names to skill-*
- [x] Verify core packages build
- [x] Create initial documentation
- [x] Generate new program keypair
- [x] Deploy to devnet for testing
- [x] Create Initialize instruction for protocol bootstrap
- [x] Add development tooling (Makefile, rustfmt, editorconfig)

**Deployed:** Program ID `3vzFzHFytiu7zkctgwX2JJhXq3XdN8J7U2WFongrejoU`

## v0.2 - Skill State Foundation (Current)
- [ ] Add skill fields to Miner state:
  - `skill_score: u64` - Accumulated skill points
  - `prediction: u8` - Current round prediction (0-24, 255=none)
  - `streak: u16` - Consecutive correct predictions
  - `last_prediction_round: u64` - Anti-replay protection
  - `challenge_count: u64` - Total attempts
  - `challenge_wins: u64` - Total wins
- [ ] Add `SubmitPrediction` instruction (discriminant 27)
- [ ] Update checkpoint to calculate and apply skill multiplier
- [ ] Add CLI commands for prediction (`predict`, `skill`)
- [ ] Cap skill multiplier at 1.5x to prevent exploitation

## v0.3 - App Integration
- [ ] Add prediction UI to web app
- [ ] Display skill scores and streaks
- [ ] Leaderboard for top skilled miners
- [ ] Mobile-friendly design improvements

## v0.4 - Anti-Gaming
- [ ] Rate limiting on predictions (1 per round)
- [ ] Sybil resistance measures
- [ ] Bot detection heuristics
- [ ] Stake-weighted skill bonuses

## v0.5 - Tokenomics Finalization
- [ ] Finalize token supply and distribution
- [ ] Implement emission schedule
- [ ] Add staking yield adjustments
- [ ] Treasury management features

## v1.0 - Mainnet Launch
- [ ] Security audit
- [ ] Final branding and naming (rename OreInstruction → SkillInstruction)
- [ ] Marketing site launch
- [ ] Token generation event
- [ ] Deploy to mainnet
- [ ] Liquidity provision

## Future Considerations
- Multiple challenge types beyond prediction
- Guild/team features
- Cross-chain expansion
- Governance token integration
