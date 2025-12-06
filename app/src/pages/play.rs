use dioxus::prelude::*;
use crate::components::Board;
use crate::hooks::{use_board, use_miner, play_transaction, claim_sol_transaction, claim_ore_transaction};
use crate::{RoundPhase, WalletState};

const INTERMISSION_SLOTS: u64 = 35;

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

#[component]
pub fn Play() -> Element {
    let wallet = use_context::<Signal<WalletState>>();
    let board = use_board();
    let miner = use_miner();

    // Multi-select squares
    let mut selected_squares = use_signal(Vec::<u8>::new);
    let mut sol_amount = use_signal(|| 0.1_f64); // Default 0.1 SOL
    let mut submitting = use_signal(|| false);
    let mut tx_result = use_signal(|| None::<Result<String, String>>);

    // Claim state
    let mut claiming_sol = use_signal(|| false);
    let mut claiming_ore = use_signal(|| false);
    let mut claim_result = use_signal(|| None::<Result<String, String>>);

    // Toggle square selection (multi-select)
    let mut toggle_square = move |square: u8| {
        let mut squares = selected_squares.write();
        if let Some(pos) = squares.iter().position(|&s| s == square) {
            squares.remove(pos);
        } else {
            squares.push(square);
        }
    };

    // Select all squares
    let select_all = move |_| {
        let mut squares = selected_squares.write();
        if squares.len() == 25 {
            squares.clear();
        } else {
            *squares = (0..25).collect();
        }
    };

    // Extract board state values
    let board_state = board.read();
    let round_id = board_state.round_id;
    let end_slot = board_state.end_slot;
    let start_slot = board_state.start_slot;
    let deployed = board_state.deployed;
    let count = board_state.count;
    let total_deployed = board_state.total_deployed;
    let current_slot = board_state.current_slot;
    let winning_square = board_state.winning_square;
    let phase = board_state.phase;
    let is_loading = board_state.loading;
    drop(board_state);

    // Calculate remaining time
    let slots_remaining = if current_slot > 0 && end_slot > current_slot {
        end_slot.saturating_sub(current_slot)
    } else {
        0
    };
    let seconds_remaining = (slots_remaining as f64 * 0.4) as u64; // ~400ms per slot

    // Check if round needs reset (for UI indication)
    let round_needs_reset = end_slot != u64::MAX && current_slot >= end_slot + INTERMISSION_SLOTS;

    // Phase-aware display
    let (time_display, time_label, timer_class) = match phase {
        RoundPhase::Deploying => {
            let class = if seconds_remaining < 10 {
                "text-red-400 font-mono text-xl animate-pulse"
            } else {
                "text-high font-mono text-xl"
            };
            (
                format!("{:02}:{:02}", seconds_remaining / 60, seconds_remaining % 60),
                "Time remaining",
                class,
            )
        }
        RoundPhase::Revealing => {
            if round_needs_reset {
                (
                    "READY".to_string(),
                    "Reset & Play",
                    "text-green-400 font-mono text-xl animate-pulse",
                )
            } else {
                (
                    "...".to_string(),
                    "Waiting for reset",
                    "text-gold font-mono text-xl animate-pulse",
                )
            }
        }
        RoundPhase::Ended => (
            format!("#{}", winning_square.map(|s| (s + 1).to_string()).unwrap_or_default()),
            "Winner",
            "text-gold font-mono text-xl",
        ),
    };

    // Miner state
    let miner_state = miner.read();
    let miner_deployed: u64 = miner_state.deployed.iter().sum();
    let rewards_sol = miner_state.rewards_sol;
    let rewards_ore = miner_state.rewards_ore;
    drop(miner_state);

    let wallet_read = wallet.read();
    let wallet_connected = wallet_read.connected;
    let wallet_pubkey = wallet_read.pubkey.clone();
    drop(wallet_read);

    rsx! {
        div { class: "w-full",
            // Two-column layout: Board | Controls
            div { class: "flex flex-col lg:flex-row gap-6",
                // Left: Game Board (wider)
                div { class: "flex-1 lg:flex-[2]",
                    Board {
                        selected: selected_squares.read().clone(),
                        winning_square: winning_square,
                        deployed: deployed,
                        count: count,
                        // Allow selection when round needs reset (player can start next round)
                        disabled: *submitting.read() || (phase != RoundPhase::Deploying && !round_needs_reset),
                        phase: phase,
                        on_select: move |square| toggle_square(square),
                        on_select_all: select_all,
                    }
                }

                // Right: Controls Panel
                div { class: "w-full lg:w-80 space-y-4",
                    // Round Info Card
                    div { class: "elevated rounded-lg p-4 elevated-border border",
                        // Round number + timer
                        div { class: "flex justify-between items-start mb-3",
                            div {
                                span { class: "text-low text-sm", "Round " }
                                span { class: "text-high font-mono text-lg", "#{round_id}" }
                            }
                            div { class: "text-right",
                                p { class: "{timer_class}", "{time_display}" }
                                p { class: "text-low text-xs", "{time_label}" }
                            }
                        }

                        // Deployed stats
                        div { class: "space-y-2 pt-3 border-t border-gray-700",
                            div { class: "flex justify-between",
                                span { class: "text-low text-sm", "Total deployed" }
                                span { class: "text-high font-mono",
                                    {format!("{:.4} SOL", total_deployed as f64 / LAMPORTS_PER_SOL)}
                                }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-low text-sm", "You deployed" }
                                span { class: "text-high font-mono",
                                    {format!("{:.4} SOL", miner_deployed as f64 / LAMPORTS_PER_SOL)}
                                }
                            }
                        }
                    }

                    // Deploy Controls Card
                    div { class: "elevated rounded-lg p-4 elevated-border border",
                        // SOL Amount buttons
                        div { class: "mb-4",
                            p { class: "text-low text-sm mb-2", "Amount" }
                            div { class: "flex gap-2",
                                button {
                                    class: if *sol_amount.read() == 1.0 { "controls-gold" } else { "elevated-control" },
                                    class: " px-3 py-1.5 rounded text-sm font-mono",
                                    onclick: move |_| sol_amount.set(1.0),
                                    "+1"
                                }
                                button {
                                    class: if *sol_amount.read() == 0.1 { "controls-gold" } else { "elevated-control" },
                                    class: " px-3 py-1.5 rounded text-sm font-mono",
                                    onclick: move |_| sol_amount.set(0.1),
                                    "+0.1"
                                }
                                button {
                                    class: if *sol_amount.read() == 0.01 { "controls-gold" } else { "elevated-control" },
                                    class: " px-3 py-1.5 rounded text-sm font-mono",
                                    onclick: move |_| sol_amount.set(0.01),
                                    "+0.01"
                                }
                            }
                        }

                        // SOL input field
                        div { class: "mb-4",
                            div { class: "flex items-center gap-2 elevated-control rounded px-3 py-2",
                                span { class: "text-gold text-lg", "◎" }
                                input {
                                    class: "bg-transparent text-high font-mono text-lg w-full outline-none",
                                    r#type: "number",
                                    step: "0.01",
                                    min: "0.01",
                                    value: "{sol_amount}",
                                    oninput: move |e| {
                                        if let Ok(val) = e.value().parse::<f64>() {
                                            sol_amount.set(val);
                                        }
                                    }
                                }
                                span { class: "text-low", "SOL" }
                            }
                        }

                        // Selection info
                        div { class: "mb-4 text-sm",
                            div { class: "flex justify-between",
                                span { class: "text-low", "Squares" }
                                span { class: "text-high font-mono",
                                    {format!("x{}", selected_squares.read().len())}
                                }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-low", "Total" }
                                span { class: "text-high font-mono",
                                    {format!("{:.4} SOL", *sol_amount.read() * selected_squares.read().len() as f64)}
                                }
                            }
                        }

                        // Play button (v0.5 - auto handles reset)
                        if !wallet_connected {
                            p { class: "text-center text-low text-sm py-2",
                                "Connect wallet to play"
                            }
                        } else {
                            // Show reset indicator when needed
                            if round_needs_reset {
                                div { class: "mb-3 p-2 bg-green-500/10 border border-green-500/30 rounded text-sm text-green-400 text-center",
                                    "Round ended - Your play will reset & start next round!"
                                }
                            }
                            button {
                                class: "w-full controls-primary py-3 rounded-lg font-semibold transition-all hover:scale-[1.02]",
                                // Allow play when round needs reset OR during active round
                                disabled: selected_squares.read().is_empty() || *submitting.read() || (phase != RoundPhase::Deploying && !round_needs_reset),
                                onclick: {
                                    let wallet_pubkey = wallet_pubkey.clone();
                                    move |_| {
                                        let pubkey = wallet_pubkey.clone();
                                        let amount = (*sol_amount.read() * LAMPORTS_PER_SOL) as u64;
                                        let squares: Vec<u8> = selected_squares.read().clone();

                                        if let Some(authority) = pubkey {
                                            submitting.set(true);
                                            tx_result.set(None);

                                            spawn(async move {
                                                // play_transaction auto-handles reset if needed
                                                let result = play_transaction(
                                                    &authority,
                                                    amount,
                                                    &squares,
                                                ).await;

                                                tx_result.set(Some(result));
                                                submitting.set(false);
                                            });
                                        }
                                    }
                                },
                                if *submitting.read() {
                                    if round_needs_reset { "Resetting & Deploying..." } else { "Deploying..." }
                                } else if selected_squares.read().is_empty() {
                                    "Select squares"
                                } else if round_needs_reset {
                                    "Reset & Play"
                                } else {
                                    "Play"
                                }
                            }
                        }

                        // Transaction result
                        if let Some(result) = tx_result.read().as_ref() {
                            match result {
                                Ok(sig) => {
                                    let explorer_url = format!("https://explorer.solana.com/tx/{}?cluster=devnet", sig);
                                    rsx! {
                                        div { class: "mt-3 p-2 bg-green-500/10 border border-green-500/30 rounded text-sm",
                                            a {
                                                href: "{explorer_url}",
                                                target: "_blank",
                                                class: "text-green-400 underline",
                                                "View transaction"
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    rsx! {
                                        div { class: "mt-3 p-2 bg-red-500/10 border border-red-500/30 rounded text-sm text-red-400",
                                            "{e}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Rewards Card
                    if wallet_connected {
                        div { class: "elevated rounded-lg p-4 elevated-border border",
                            h3 { class: "text-gold font-semibold mb-3", "Rewards" }

                            // SOL Rewards
                            div { class: "flex justify-between items-center mb-3",
                                div {
                                    span { class: "text-low text-sm", "SOL" }
                                    p { class: "text-high font-mono",
                                        {format!("{:.6}", rewards_sol as f64 / LAMPORTS_PER_SOL)}
                                    }
                                }
                                button {
                                    class: "controls-gold px-4 py-1.5 rounded text-sm font-semibold",
                                    disabled: rewards_sol == 0 || *claiming_sol.read(),
                                    onclick: {
                                        let wallet_pubkey = wallet_pubkey.clone();
                                        move |_| {
                                            if let Some(authority) = wallet_pubkey.clone() {
                                                claiming_sol.set(true);
                                                claim_result.set(None);

                                                spawn(async move {
                                                    let result = claim_sol_transaction(&authority).await;
                                                    claim_result.set(Some(result));
                                                    claiming_sol.set(false);
                                                });
                                            }
                                        }
                                    },
                                    if *claiming_sol.read() { "Claiming..." } else { "Claim SOL" }
                                }
                            }

                            // SKILL Token Rewards
                            div { class: "flex justify-between items-center",
                                div {
                                    span { class: "text-low text-sm", "SKILL" }
                                    p { class: "text-high font-mono",
                                        {format!("{:.2}", rewards_ore as f64 / 100_000_000_000.0)}
                                    }
                                }
                                button {
                                    class: "controls-gold px-4 py-1.5 rounded text-sm font-semibold",
                                    disabled: rewards_ore == 0 || *claiming_ore.read(),
                                    onclick: {
                                        let wallet_pubkey = wallet_pubkey.clone();
                                        move |_| {
                                            if let Some(authority) = wallet_pubkey.clone() {
                                                claiming_ore.set(true);
                                                claim_result.set(None);

                                                spawn(async move {
                                                    let result = claim_ore_transaction(&authority).await;
                                                    claim_result.set(Some(result));
                                                    claiming_ore.set(false);
                                                });
                                            }
                                        }
                                    },
                                    if *claiming_ore.read() { "Claiming..." } else { "Claim SKILL" }
                                }
                            }

                            // Claim result
                            if let Some(result) = claim_result.read().as_ref() {
                                match result {
                                    Ok(sig) => {
                                        let explorer_url = format!("https://explorer.solana.com/tx/{}?cluster=devnet", sig);
                                        rsx! {
                                            div { class: "mt-3 p-2 bg-green-500/10 border border-green-500/30 rounded text-sm",
                                                a {
                                                    href: "{explorer_url}",
                                                    target: "_blank",
                                                    class: "text-green-400 underline",
                                                    "View claim transaction"
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        rsx! {
                                            div { class: "mt-3 p-2 bg-red-500/10 border border-red-500/30 rounded text-sm text-red-400",
                                                "{e}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
