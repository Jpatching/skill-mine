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

## v0.2 - Skill State Foundation ✅ COMPLETE
- [x] Add skill fields to Miner state:
  - `skill_score: u64` - Accumulated skill points
  - `prediction: u8` - Current round prediction (0-24, 255=none)
  - `streak: u16` - Consecutive correct predictions
  - `last_prediction_round: u64` - Anti-replay protection
  - `challenge_count: u64` - Total attempts
  - `challenge_wins: u64` - Total wins
- [x] Add `SubmitPrediction` instruction (discriminant 27)
- [x] Update checkpoint to calculate and apply skill multiplier
- [x] Add CLI commands for prediction (`predict`, `skill`)
- [x] Cap skill multiplier at 1.5x to prevent exploitation

**Implementation:** Skill multiplier formula: `base 1.0x + log10(score)*5% + streak*2%` (capped at 1.5x)

## v0.3 - App Integration ✅ COMPLETE
- [x] Add prediction UI to web app (Dioxus 0.6 full-stack)
- [x] Display skill scores and streaks
- [x] Leaderboard for top skilled miners (Helius API integration)
- [x] Phantom wallet integration for transaction signing
- [x] Web + Desktop support via Dioxus

**Stack:** Dioxus 0.6 + Tailwind CSS + Phantom Wallet Adapter

## v0.4 - Schelling Point Design ✅ COMPLETE
- [x] Remove entropy dependency from deploy instruction
- [x] Remove entropy dependency from reset instruction
- [x] Winner determined by majority vote (argmax of deployed SOL)
- [x] Update SDK to not require entropy accounts
- [x] Update CLI reset command
- [x] Update app deploy hook
- [x] Deploy and verify on devnet

**Key Change:** Winner is now the square with the most SOL deployed (coordination game).
No external randomness required. Tie-breaker: lower index wins.

**Deployed:** Program redeployed to devnet with Schelling Point logic.

## v0.5 - Schelling Point Core ✅ COMPLETE
- [x] Fix Square 0 win bug (winning_square stored directly)
- [x] Sample slot_hash from Solana for split/motherlode RNG
- [x] Add `winning_square` field to Round state
- [x] Update checkpoint to use stored winning_square
- [x] Create `play()` SDK function (reset+deploy bundle)
- [x] Add `play` CLI command (auto-reset if needed)
- [x] Remove entropy dependency completely
- [x] Self-cranking game loop (players are the crank)

**Key Changes:**
- Winner determined by `argmax(deployed)` - no external RNG
- Split/motherlode use Solana slot_hash for unpredictability
- `play` command bundles reset+deploy in one transaction
- Zero external infrastructure needed

## v0.6 - Schelling Point Enhancements (Next)
- [ ] Commit-reveal scheme to prevent last-second copying
- [ ] Confidence multiplier (higher stake = more conviction)
- [ ] Rating system based on coordination success rate
- [ ] Natural focal points UI hints (center, corners)
- [ ] Round history showing coordination patterns
- [ ] Longer rounds (5-10 min) for better coordination

## v0.7 - Anti-Gaming
- [ ] Rate limiting on predictions (1 per round)
- [ ] Sybil resistance measures
- [ ] Bot detection heuristics
- [ ] Stake-weighted skill bonuses

## v0.8 - Tokenomics Finalization
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
