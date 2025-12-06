#![allow(non_snake_case)]

mod components;
mod hooks;
mod pages;
mod route;

use dioxus::prelude::*;
use route::Route;

// Configuration
pub const PROGRAM_ID: &str = "3vzFzHFytiu7zkctgwX2JJhXq3XdN8J7U2WFongrejoU";
pub const RPC_URL: &str = "https://api.devnet.solana.com";
pub const HELIUS_API_KEY: &str = "500713ba-c589-40c0-babd-d7d77c62ffff";

// PDA seeds (matching skill-api)
pub const BOARD_SEED: &[u8] = b"board";
pub const MINER_SEED: &[u8] = b"miner";
pub const ROUND_SEED: &[u8] = b"round";

fn main() {
    #[cfg(feature = "web")]
    {
        tracing_wasm::set_as_global_default();
        dioxus::launch(App);
    }

    #[cfg(feature = "desktop")]
    {
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {
    // Global state providers
    use_context_provider(|| Signal::new(WalletState::default()));
    use_context_provider(|| Signal::new(BoardState::default()));
    use_context_provider(|| Signal::new(MinerState::default()));

    rsx! {
        Router::<Route> {}
    }
}

// Global state types
#[derive(Clone, Default, Debug)]
pub struct WalletState {
    pub connected: bool,
    pub pubkey: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RoundPhase {
    #[default]
    Deploying,  // Round active, accepting deployments
    Revealing,  // Round ended, waiting for entropy/reset
    Ended,      // Winner determined, awaiting new round
}

#[derive(Clone, Debug)]
pub struct BoardState {
    pub round_id: u64,
    pub start_slot: u64,
    pub end_slot: u64,
    pub deployed: [u64; 25],        // SOL deployed per square (lamports)
    pub count: [u64; 25],           // Miner count per square
    pub total_deployed: u64,        // Total SOL in round
    pub current_slot: u64,          // Current slot for timer calculation
    pub winning_square: Option<u8>, // Set when round ends
    pub phase: RoundPhase,          // Current round phase
    pub loading: bool,
}

impl Default for BoardState {
    fn default() -> Self {
        Self {
            round_id: 0,
            start_slot: 0,
            end_slot: u64::MAX,
            deployed: [0; 25],
            count: [0; 25],
            total_deployed: 0,
            current_slot: 0,
            winning_square: None,
            phase: RoundPhase::Deploying,
            loading: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MinerState {
    pub deployed: [u64; 25],        // User's SOL deployed per square (lamports)
    pub skill_score: u64,
    pub streak: u16,
    pub prediction: Option<u8>,
    pub challenge_count: u64,
    pub challenge_wins: u64,
    pub rewards_sol: u64,
    pub rewards_ore: u64,
    pub loading: bool,
}

impl Default for MinerState {
    fn default() -> Self {
        Self {
            deployed: [0; 25],
            skill_score: 0,
            streak: 0,
            prediction: None,
            challenge_count: 0,
            challenge_wins: 0,
            rewards_sol: 0,
            rewards_ore: 0,
            loading: true,
        }
    }
}
