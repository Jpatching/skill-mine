use dioxus::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use crate::{BoardState, RoundPhase, RPC_URL};
use super::rpc::{fetch_account, fetch_slot, board_pda, round_pda};

pub fn use_board() -> Signal<BoardState> {
    let board = use_context::<Signal<BoardState>>();

    // Track if polling has started to prevent multiple loops
    let polling_started = use_hook(|| Rc::new(Cell::new(false)));

    // Start polling only once
    use_effect(move || {
        if !polling_started.get() {
            polling_started.set(true);

            spawn(async move {
                loop {
                    // Fetch board data
                    if let Err(e) = fetch_and_update_board_safe(board).await {
                        tracing::error!("Board fetch error: {}", e);
                    }

                    // Adaptive polling interval
                    let poll_interval = {
                        let state = board.read();
                        let slots_remaining = state.end_slot.saturating_sub(state.current_slot);
                        let seconds_remaining = (slots_remaining as f64 * 0.4) as u64;

                        match state.phase {
                            RoundPhase::Deploying if seconds_remaining < 15 => 1500,
                            RoundPhase::Deploying => 3000,
                            RoundPhase::Committing => 2000,
                            RoundPhase::Revealing => 2000,
                            RoundPhase::Ended => 5000,
                        }
                    };

                    gloo_timers::future::TimeoutFuture::new(poll_interval).await;
                }
            });
        }
    });

    board
}

async fn fetch_and_update_board_safe(mut board: Signal<BoardState>) -> Result<(), String> {
    let data = fetch_board_and_round().await?;
    let mut board_mut = board.write();
    board_mut.round_id = data.round_id;
    board_mut.start_slot = data.start_slot;
    board_mut.end_slot = data.end_slot;
    board_mut.deployed = data.deployed;
    board_mut.count = data.count;
    board_mut.total_deployed = data.total_deployed;
    board_mut.current_slot = data.current_slot;
    board_mut.winning_square = data.winning_square;
    board_mut.phase = data.phase;
    board_mut.bonus_squares = data.bonus_squares;
    board_mut.commit_start_slot = data.commit_start_slot;
    board_mut.reveal_start_slot = data.reveal_start_slot;
    board_mut.loading = false;
    Ok(())
}

async fn fetch_and_update_board(mut board: Signal<BoardState>) {
    match fetch_board_and_round().await {
        Ok(data) => {
            let mut board_mut = board.write();
            board_mut.round_id = data.round_id;
            board_mut.start_slot = data.start_slot;
            board_mut.end_slot = data.end_slot;
            board_mut.deployed = data.deployed;
            board_mut.count = data.count;
            board_mut.total_deployed = data.total_deployed;
            board_mut.current_slot = data.current_slot;
            board_mut.winning_square = data.winning_square;
            board_mut.phase = data.phase;
            board_mut.loading = false;
        }
        Err(e) => {
            tracing::error!("Failed to fetch board: {}", e);
            board.write().loading = false;
        }
    }
}

#[derive(Default)]
struct BoardData {
    round_id: u64,
    start_slot: u64,
    end_slot: u64,
    deployed: [u64; 25],
    count: [u64; 25],
    total_deployed: u64,
    current_slot: u64,
    winning_square: Option<u8>,
    phase: RoundPhase,
    bonus_squares: [u8; 3],
    commit_start_slot: u64,
    reveal_start_slot: u64,
}

async fn fetch_board_and_round() -> Result<BoardData, String> {
    // First fetch Board to get current round_id
    let board_pda = board_pda();
    let board_bytes = fetch_account(RPC_URL, &board_pda).await?;

    let mut data = BoardData::default();

    if let Some(bytes) = board_bytes {
        if bytes.len() >= 32 {
            data.round_id = u64::from_le_bytes(bytes[8..16].try_into().unwrap_or_default());
            data.start_slot = u64::from_le_bytes(bytes[16..24].try_into().unwrap_or_default());
            data.end_slot = u64::from_le_bytes(bytes[24..32].try_into().unwrap_or_default());
        }
    }

    // Fetch current slot for timer calculation
    if let Ok(slot) = fetch_slot(RPC_URL).await {
        data.current_slot = slot;
    }

    // Then fetch Round to get deployments and counts
    // Round ID 0 is valid - it's the first round after init
    let round_pda = round_pda(data.round_id);
    if let Ok(Some(round_bytes)) = fetch_account(RPC_URL, &round_pda).await {
        // Round layout (after 8-byte discriminator):
        // id: u64 (8 bytes) - offset 8
        // deployed: [u64; 25] (200 bytes) - offset 16
        // slot_hash: [u8; 32] - offset 216
        // count: [u64; 25] (200 bytes) - offset 248
        // expires_at: u64 - offset 448
        // motherlode: u64 - offset 456
        // rent_payer: Pubkey (32) - offset 464
        // top_miner: Pubkey (32) - offset 496
        // top_miner_reward: u64 - offset 528
        // total_deployed: u64 - offset 536
        // total_vaulted: u64 - offset 544
        // total_winnings: u64 - offset 552
        // winning_square: u8 - offset 560
        // bonus_squares: [u8; 3] - offset 561 (v0.6)
        // _padding: [u8; 4] - offset 564
        // commit_start_slot: u64 - offset 568 (v0.6)
        // reveal_start_slot: u64 - offset 576 (v0.6)
        // revealed_count: [u64; 25] (200 bytes) - offset 584 (v0.6)
        // total_reveals: u64 - offset 784 (v0.6)
        if round_bytes.len() >= 216 {
            // Parse deployed array
            for i in 0..25 {
                let offset = 16 + i * 8;
                data.deployed[i] = u64::from_le_bytes(
                    round_bytes[offset..offset + 8].try_into().unwrap_or_default()
                );
            }
            data.total_deployed = data.deployed.iter().sum();

            // Check if round has been finalized (slot_hash is set during reset)
            let slot_hash_offset = 216;
            let slot_hash: [u8; 32] = round_bytes[slot_hash_offset..slot_hash_offset + 32]
                .try_into()
                .unwrap_or([0; 32]);

            // winning_square is stored at offset 560
            if slot_hash != [0u8; 32] && round_bytes.len() >= 561 {
                data.winning_square = Some(round_bytes[560]);
            }

            // v0.6: Parse bonus_squares [u8; 3] at offset 561
            if round_bytes.len() >= 564 {
                data.bonus_squares = [
                    round_bytes[561],
                    round_bytes[562],
                    round_bytes[563],
                ];
            }

            // v0.6: Parse commit/reveal slots at offsets 568, 576
            if round_bytes.len() >= 584 {
                data.commit_start_slot = u64::from_le_bytes(
                    round_bytes[568..576].try_into().unwrap_or_default()
                );
                data.reveal_start_slot = u64::from_le_bytes(
                    round_bytes[576..584].try_into().unwrap_or_default()
                );
            }

            // Parse count array (offset 248, 200 bytes)
            if round_bytes.len() >= 448 {
                for i in 0..25 {
                    let offset = 248 + i * 8;
                    data.count[i] = u64::from_le_bytes(
                        round_bytes[offset..offset + 8].try_into().unwrap_or_default()
                    );
                }
            }
        }
    }

    // Calculate round phase based on commit-reveal timing
    // Flow: Deploying → Committing → Revealing → Ended
    data.phase = if data.winning_square.is_some() {
        // Round finalized - winner determined
        RoundPhase::Ended
    } else if data.reveal_start_slot > 0 && data.current_slot >= data.reveal_start_slot {
        // Past reveal start - in reveal phase
        RoundPhase::Revealing
    } else if data.commit_start_slot > 0 && data.current_slot >= data.commit_start_slot {
        // Past commit start but before reveal - in commit phase
        // During this phase, users submit choice hash (visible SOL but hidden choice)
        RoundPhase::Committing
    } else {
        // Default: deploying phase (SOL deployment visible, choices not yet locked)
        RoundPhase::Deploying
    };

    Ok(data)
}
