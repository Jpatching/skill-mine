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

## v0.3 App Implementation Learnings

### Dioxus 0.6 Full-Stack Architecture
Following ORE's pattern with Dioxus for both web and desktop:

```
app/
├── Cargo.toml           # Separate workspace (not part of main)
├── Dioxus.toml          # Framework configuration
├── src/
│   ├── main.rs          # Entry point with global providers
│   ├── route.rs         # Page routing
│   ├── components/      # Reusable UI (Board, WalletButton, etc.)
│   ├── hooks/           # State management (use_board, use_miner, etc.)
│   └── pages/           # Route pages (Home, Play, Leaderboard, Stats)
```

### Key Dioxus Patterns

**Global State with Providers:**
```rust
fn App() -> Element {
    use_context_provider(|| Signal::new(WalletState::default()));
    use_context_provider(|| Signal::new(BoardState::default()));

    rsx! { Router::<Route> {} }
}
```

**Signal Mutability:**
```rust
// Correct - must be mut for write access
let mut wallet = use_context::<Signal<WalletState>>();
wallet.write().connected = true;

// Extract primitives for RSX to avoid borrow issues
let wallet_connected = wallet.read().connected;
rsx! { if wallet_connected { ... } }
```

**Conditional Compilation for Web/Desktop:**
```rust
#[cfg(feature = "web")]
async fn connect_phantom() -> Result<String, String> {
    use wasm_bindgen::prelude::*;
    // Browser-specific JS interop
}

#[cfg(not(feature = "web"))]
async fn connect_phantom() -> Result<String, String> {
    Err("Phantom only available in web mode".to_string())
}
```

### Phantom Wallet Integration

**Connection via window.solana:**
```rust
use js_sys::{Reflect, Promise};

let window = web_sys::window().ok_or("No window")?;
let solana = Reflect::get(&window, &JsValue::from_str("solana"))?;

// Check is Phantom
let is_phantom = Reflect::get(&solana, &JsValue::from_str("isPhantom"))?;

// Call connect()
let connect_fn: js_sys::Function = Reflect::get(&solana, &JsValue::from_str("connect"))?.dyn_into()?;
let promise: Promise = connect_fn.call0(&solana)?.dyn_into()?;
let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
```

**Transaction Signing:**
- Transactions sent as base64-encoded bytes
- Phantom's `signAndSendTransaction` handles signing + broadcast
- Returns transaction signature on success

### Helius API for Leaderboard

**Fetching Program Accounts:**
```rust
let request = json!({
    "jsonrpc": "2.0",
    "id": "skill-leaderboard",
    "method": "getProgramAccounts",
    "params": {
        "programId": PROGRAM_ID,
        "encoding": "base64",
        "filters": [{ "dataSize": 544 }]  // Miner account size
    }
});
```

**Parsing Miner Data:**
- Decode base64 account data
- Parse skill fields at known offsets:
  - `skill_score`: bytes 496-504
  - `streak`: bytes 506-508
  - `challenge_count`: bytes 520-528
  - `challenge_wins`: bytes 528-536

### RSX String Formatting Gotchas

```rust
// Works - interpolation with owned String
let msg = format!("Score: {}", score);
rsx! { p { "{msg}" } }

// Works - inline format! macro
rsx! { p { {format!("Score: {}", score)} } }

// Doesn't work - direct str slice in interpolation
rsx! { p { "Score: {some_str_slice}" } }  // Error!
```

### Tailwind CSS with Dioxus

**Configuration (Dioxus.toml):**
```toml
[web.watcher]
watch_path = ["src", "public"]

[web.resource]
style = ["/tailwind.css"]
```

**Build command:**
```bash
npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch
```

### Desktop vs Web Considerations

- Desktop uses native window, no browser APIs
- Feature flags control compilation: `#[cfg(feature = "web")]`
- Wallet connection only works in web mode
- Desktop could use local keypair file instead

### Development Workflow

```bash
# Terminal 1: Tailwind watcher
cd app && npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch

# Terminal 2: Dioxus dev server
cd app && dx serve

# For desktop
cd app && dx serve --platform desktop
```

## v0.4 Schelling Point Design

### What Changed from RNG to Coordination

**Before (Entropy-based):**
- Deploy SOL to squares during round
- At round end, external entropy program provides randomness
- Winning square = `entropy_value % 25`
- Winners share losers' SOL

**After (Schelling Point):**
- Deploy SOL to squares during round (same)
- At round end, winning square = `argmax(deployed)` (most SOL wins)
- No external entropy required
- Creates coordination game - players try to coordinate on same square

### Game Theory Implications

**Schelling Point** (Thomas Schelling, 1960): In coordination games without communication,
players often converge on "focal points" - solutions that seem natural or special.

**Natural Focal Points in SKILL:**
- Square 12 (center of 5x5 grid)
- Square 0 (first/origin)
- Square 24 (last)
- Corners (0, 4, 20, 24)

**Strategy Evolution:**
1. Early rounds: Players experiment, find focal points
2. Mid-game: Dominant focal point emerges
3. Late-game: Counter-play opportunities (bet against majority)

### Implementation Details

**Winner Determination (`reset.rs`):**
```rust
// Find winning square = argmax(deployed)
let (winning_square, max_deployed) = round
    .deployed
    .iter()
    .enumerate()
    .max_by(|(i1, v1), (i2, v2)| {
        // Primary: most SOL deployed
        // Secondary: lower index (for deterministic tie-breaking)
        v1.cmp(v2).then_with(|| i2.cmp(i1))
    })
    .map(|(i, &v)| (i, v))
    .unwrap_or((0, 0));
```

**Key Changes:**
- `deploy.rs`: Removed entropy account validation and CPI call
- `reset.rs`: Removed entropy sampling, uses argmax instead
- `sdk.rs`: Removed entropy accounts from instruction builders
- `cli/main.rs`: Simplified reset command

### Files Modified for Schelling Point

| File | Change |
|------|--------|
| `program/src/deploy.rs` | Removed entropy accounts, creates Round 0 if needed |
| `program/src/reset.rs` | Winner = argmax(deployed), no entropy sampling |
| `api/src/sdk.rs` | Deploy/reset no longer include entropy accounts |
| `cli/src/main.rs` | Reset shows winning square before transaction |
| `app/src/hooks/use_deploy.rs` | Removed entropy accounts from tx building |

### Verification Commands

```bash
# 1. Check board state
KEYPAIR=~/.config/solana/id.json RPC=https://api.devnet.solana.com \
  COMMAND=board cargo run -p skill-cli

# 2. Deploy to a square (starts round if none active)
KEYPAIR=~/.config/solana/id.json RPC=https://api.devnet.solana.com \
  COMMAND=deploy SQUARE=12 AMOUNT=1000000 cargo run -p skill-cli

# 3. Check round state
KEYPAIR=~/.config/solana/id.json RPC=https://api.devnet.solana.com \
  COMMAND=round ID=<round_id> cargo run -p skill-cli

# 4. Reset after round ends (determines winner by majority)
KEYPAIR=~/.config/solana/id.json RPC=https://api.devnet.solana.com \
  COMMAND=reset cargo run -p skill-cli

# 5. Run full verification script
./scripts/verify-schelling-point.sh
```

### Future Enhancements

**Commit-Reveal Scheme:**
- Phase 1: Players submit `hash(square + salt)`
- Phase 2: Players reveal `square + salt`
- Prevents last-second copying of majority

**Confidence Multiplier:**
- Higher stake = more "conviction" in choice
- Rewards players who commit early and large

**Rating System:**
- Track coordination success rate
- Players who consistently find focal points get bonus
- Creates skill-based differentiation

## v0.5 Implementation Learnings

### Square 0 Bug Fix
Original design stored winning_square in `slot_hash[0]`. Problem: if square 0 wins, slot_hash == [0;32], which checkpoint treated as "invalid round".

**Solution:** Add dedicated `winning_square: u8` field to Round state.

### Slot Hash RNG (No Entropy Needed)
Solana provides unpredictable slot hashes via `SlotHashes` sysvar:
```rust
let slot_hashes: SlotHashes = bincode::deserialize(&slot_hashes_sysvar.data)?;
round.slot_hash = slot_hashes.get(&board.end_slot).to_bytes();
```

This gives us unpredictable RNG for split/motherlode without external dependencies.

### Self-Cranking Game Loop
Key insight: ORE doesn't run a crank. The miners ARE the crank.

For SKILL, players call reset when they want to play next round:
```rust
pub fn play(..., round_ended: bool) -> Vec<Instruction> {
    let mut ixs = vec![];
    if round_ended {
        ixs.push(reset(...));  // Finalize current round
    }
    ixs.push(deploy(...));     // Join next round
    ixs
}
```

**Benefits:**
- Zero infrastructure cost
- No external bots needed
- Players pay their own gas (as they should)
- Game pauses naturally when no one plays

### Round State Extension
Adding fields to existing accounts requires:
1. Place new fields at END of struct
2. Add padding for alignment (Pod derive requires 8-byte alignment)
3. Redeploy program (devnet) or migrate accounts (mainnet)

```rust
pub struct Round {
    // ... existing 45 fields ...
    pub winning_square: u8,   // NEW
    pub _padding: [u8; 7],    // Alignment
}
```

### Account Migration for Existing Rounds
When Round struct size changes (560 → 568 bytes), existing accounts need migration:

```rust
// In deploy.rs - handle old accounts with smaller size
} else if round_info.data_len() < expected_size {
    // Transfer additional rent from signer BEFORE resizing
    let rent = Rent::get()?;
    let diff = rent.minimum_balance(expected_size) - round_info.lamports();
    round_info.collect(diff, signer_info)?;  // Use system program

    // Resize and initialize new fields
    round_info.resize(expected_size)?;
    let mut data = round_info.try_borrow_mut_data()?;
    data[560] = 0;  // winning_square
    data[561..568].copy_from_slice(&[0; 7]);  // _padding
}
```

### Checkpoint Requirement in Play Loop
Miners must checkpoint previous round before deploying to new round:

```rust
// In sdk.rs play() function
if round_ended {
    instructions.push(reset(...));      // 1. Finalize current round
    instructions.push(checkpoint(...)); // 2. Claim rewards from that round
}
instructions.push(deploy(...));         // 3. Join next round
```

The deploy instruction enforces this with:
```rust
assert!(miner.checkpoint_id == miner.round_id, "Miner has not checkpointed");
```

### Slot Hash without Bincode
`bincode` crate doesn't work in SBF environment. Instead, hash raw sysvar data:

```rust
let slot_hashes_data = slot_hashes_sysvar.data.borrow();
round.slot_hash = keccak::hashv(&[
    &slot_hashes_data[..slot_hashes_data.len().min(256)],
    &board.end_slot.to_le_bytes(),
    &round.total_deployed.to_le_bytes(),
    &clock.slot.to_le_bytes(),
]).to_bytes();
```

### Verification of Full Game Loop

Tested commands showing working game:
```bash
# 1. Deploy to start round
COMMAND=play SQUARE=12 AMOUNT=10000000 cargo run -p skill-cli
# Output: Round 3 active, deployed 0.01 SOL to square 12

# 2. Wait for round to end (~60 seconds)

# 3. Play again (triggers reset + checkpoint + deploy)
COMMAND=play SQUARE=8 AMOUNT=20000000 cargo run -p skill-cli
# Output: Round 3 ended. Resetting...
#         Winning square: #12 (10000000 lamports)
#         Transaction submitted

# 4. Verify miner has rewards
COMMAND=miner cargo run -p skill-cli
# Output: rewards_sol: 0.033561 SOL, rewards_ore: 4 ORE

# 5. Claim rewards
COMMAND=claim cargo run -p skill-cli
# Output: Transaction submitted (rewards transferred to wallet)
```
