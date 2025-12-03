# SKILL-MINE Roadmap

## v0.1 - Clean Fork (Current)
- [x] Clone fresh ORE repositories (program, CLI, app)
- [x] Rebrand package names to skill-*
- [x] Verify core packages build
- [x] Create initial documentation
- [ ] Generate new program keypair
- [ ] Deploy to devnet for testing

## v0.2 - Skill State Foundation
- [ ] Add skill fields to Miner state:
  - `skill_score: u64` - Accumulated skill points
  - `prediction: u8` - Current round prediction (0-24, 255=none)
  - `streak: u16` - Consecutive correct predictions
  - `last_challenge_round: u64` - Anti-spam tracking
- [ ] Add `SubmitPrediction` instruction
- [ ] Update checkpoint to calculate skill multiplier
- [ ] Add CLI commands for prediction

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
- [ ] Final branding and naming
- [ ] Marketing site launch
- [ ] Token generation event
- [ ] Deploy to mainnet
- [ ] Liquidity provision

## Future Considerations
- Multiple challenge types beyond prediction
- Guild/team features
- Cross-chain expansion
- Governance token integration
