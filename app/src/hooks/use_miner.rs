use dioxus::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use crate::{MinerState, WalletState, RPC_URL};
use super::rpc::{fetch_account, miner_pda};

pub fn use_miner() -> Signal<MinerState> {
    let miner = use_context::<Signal<MinerState>>();
    let wallet = use_context::<Signal<WalletState>>();

    // Track if polling has started to prevent multiple loops
    let polling_started = use_hook(|| Rc::new(Cell::new(false)));

    // Start polling only once
    use_effect(move || {
        if !polling_started.get() {
            polling_started.set(true);
            let mut miner = miner;

            spawn(async move {
                loop {
                    let pubkey = wallet.read().pubkey.clone();
                    if let Some(authority) = pubkey {
                        match fetch_miner_data(&authority).await {
                            Ok(data) => {
                                let mut miner_mut = miner.write();
                                miner_mut.deployed = data.deployed;
                                miner_mut.skill_score = data.skill_score;
                                miner_mut.streak = data.streak;
                                miner_mut.prediction = data.prediction;
                                miner_mut.challenge_count = data.challenge_count;
                                miner_mut.challenge_wins = data.challenge_wins;
                                miner_mut.rewards_sol = data.rewards_sol;
                                miner_mut.rewards_ore = data.rewards_ore;
                                miner_mut.loading = false;
                            }
                            Err(e) => {
                                tracing::error!("Failed to fetch miner: {}", e);
                            }
                        }
                    }
                    // Poll every 4 seconds (offset from board poll)
                    gloo_timers::future::TimeoutFuture::new(4000).await;
                }
            });
        }
    });

    miner
}

#[derive(Default)]
struct MinerData {
    deployed: [u64; 25],
    skill_score: u64,
    streak: u16,
    prediction: Option<u8>,
    challenge_count: u64,
    challenge_wins: u64,
    rewards_sol: u64,
    rewards_ore: u64,
}

async fn fetch_miner_data(authority: &str) -> Result<MinerData, String> {
    let pda = miner_pda(authority);
    let data = fetch_account(RPC_URL, &pda).await?;

    if let Some(bytes) = data {
        // Parse Miner account (matching api/src/state/miner.rs layout)
        // Layout:
        // 0-8: discriminator
        // 8-40: authority (32 bytes)
        // 40-240: deployed [u64; 25] (200 bytes)
        // 240-440: cumulative [u64; 25] (200 bytes)
        // 440-448: checkpoint_fee (u64)
        // 448-456: checkpoint_id (u64)
        // 456-464: lifetime_rewards_ore (u64)
        // 464-472: lifetime_rewards_sol (u64)
        // 472-480: rewards_ore (u64)
        // 480-488: rewards_sol (u64)
        // 488-496: round_id (u64)
        // 496-504: skill_score (u64)
        // 504-505: prediction (u8)
        // 505-506: _padding1
        // 506-508: streak (u16)
        // 508-512: _padding2
        // 512-520: last_prediction_round (u64)
        // 520-528: challenge_count (u64)
        // 528-536: challenge_wins (u64)

        if bytes.len() >= 536 {
            // Parse deployed array from bytes 40-240 (25 * 8 bytes)
            let mut deployed = [0u64; 25];
            for i in 0..25 {
                let offset = 40 + (i * 8);
                deployed[i] =
                    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap_or_default());
            }

            let rewards_ore = u64::from_le_bytes(bytes[472..480].try_into().unwrap_or_default());
            let rewards_sol = u64::from_le_bytes(bytes[480..488].try_into().unwrap_or_default());
            let skill_score = u64::from_le_bytes(bytes[496..504].try_into().unwrap_or_default());
            let prediction_raw = bytes[504];
            let prediction = if prediction_raw == 255 { None } else { Some(prediction_raw) };
            let streak = u16::from_le_bytes(bytes[506..508].try_into().unwrap_or_default());
            let challenge_count = u64::from_le_bytes(bytes[520..528].try_into().unwrap_or_default());
            let challenge_wins = u64::from_le_bytes(bytes[528..536].try_into().unwrap_or_default());

            return Ok(MinerData {
                deployed,
                skill_score,
                streak,
                prediction,
                challenge_count,
                challenge_wins,
                rewards_sol,
                rewards_ore,
            });
        }
    }

    Ok(MinerData::default())
}
