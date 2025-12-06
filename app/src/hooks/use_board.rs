use dioxus::prelude::*;
use crate::{BoardState, RoundPhase, RPC_URL};
use super::rpc::{fetch_account, fetch_slot, board_pda, round_pda};

pub fn use_board() -> Signal<BoardState> {
    let mut board = use_context::<Signal<BoardState>>();

    // Use future for polling - automatically cancelled on component unmount
    use_future(move || async move {
        loop {
            fetch_and_update_board(board).await;

            // Adaptive polling based on round state
            let poll_interval = {
                let state = board.read();
                let slots_remaining = state.end_slot.saturating_sub(state.current_slot);
                let seconds_remaining = (slots_remaining as f64 * 0.4) as u64;

                match state.phase {
                    RoundPhase::Deploying if seconds_remaining < 15 => 500,  // Fast poll near end
                    RoundPhase::Deploying => 1000,                           // Normal active poll
                    RoundPhase::Revealing => 500,                            // Fast poll during reveal
                    RoundPhase::Ended => 3000,                               // Slow poll when ended
                }
            };

            gloo_timers::future::TimeoutFuture::new(poll_interval).await;
        }
    });

    board
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
        // winning_square: u8 - offset 560 (NEW in v0.5)
        // _padding: [u8; 7] - offset 561
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

            // v0.5: winning_square is now stored directly at offset 560
            // (not in slot_hash[0] anymore - fixes square 0 bug)
            if slot_hash != [0u8; 32] && round_bytes.len() >= 561 {
                data.winning_square = Some(round_bytes[560]);
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

    // Calculate round phase
    data.phase = if data.winning_square.is_some() {
        RoundPhase::Ended
    } else if data.current_slot >= data.end_slot {
        RoundPhase::Revealing
    } else {
        RoundPhase::Deploying
    };

    Ok(data)
}
