use dioxus::prelude::*;
use crate::RoundPhase;

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

#[derive(Props, Clone, PartialEq)]
pub struct BoardProps {
    /// Currently selected squares (multi-select)
    #[props(default)]
    pub selected: Vec<u8>,
    /// Winning square (if round ended)
    #[props(default)]
    pub winning_square: Option<u8>,
    /// SOL deployed per square (in lamports)
    #[props(default)]
    pub deployed: [u64; 25],
    /// Miner count per square
    #[props(default)]
    pub count: [u64; 25],
    /// Callback when square is clicked
    #[props(default)]
    pub on_select: Option<EventHandler<u8>>,
    /// Callback for "Select all" button
    #[props(default)]
    pub on_select_all: Option<EventHandler<()>>,
    /// Whether board is disabled
    #[props(default = false)]
    pub disabled: bool,
    /// Current round phase for animations
    #[props(default)]
    pub phase: RoundPhase,
    /// Bonus squares (highlighted with star)
    #[props(default)]
    pub bonus_squares: [u8; 3],
}

#[component]
pub fn Board(props: BoardProps) -> Element {
    // Calculate total deployed and find leading square
    let total_deployed: u64 = props.deployed.iter().sum();
    let (leading_square, max_deployed) = props.deployed
        .iter()
        .enumerate()
        .max_by_key(|(_, &v)| v)
        .map(|(i, &v)| (i as u8, v))
        .unwrap_or((0, 0));

    rsx! {
        div { class: "space-y-3",
            // 5x5 Grid - ORE style with heat map
            div { class: "grid grid-cols-5 gap-1.5",
                for i in 0..25u8 {
                    Square {
                        index: i,
                        selected: props.selected.contains(&i),
                        winning: props.winning_square == Some(i),
                        leading: leading_square == i && max_deployed > 0 && props.winning_square.is_none(),
                        deployed: props.deployed[i as usize],
                        total_deployed: total_deployed,
                        max_deployed: max_deployed,
                        count: props.count[i as usize],
                        disabled: props.disabled,
                        phase: props.phase,
                        is_bonus: props.bonus_squares.contains(&i),
                        on_click: move |_| {
                            if let Some(handler) = &props.on_select {
                                if !props.disabled {
                                    handler.call(i);
                                }
                            }
                        },
                    }
                }
            }

            // Select all button
            div { class: "flex items-center justify-center pt-2",
                button {
                    class: "flex items-center gap-2 text-sm text-low hover:text-mid transition-colors",
                    disabled: props.disabled,
                    onclick: move |_| {
                        if let Some(handler) = &props.on_select_all {
                            handler.call(());
                        }
                    },
                    // Grid icon
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        view_box: "0 0 24 24",
                        path {
                            d: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z"
                        }
                    }
                    "Select all"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct SquareProps {
    index: u8,
    selected: bool,
    winning: bool,
    leading: bool,
    deployed: u64,
    total_deployed: u64,
    max_deployed: u64,
    count: u64,
    disabled: bool,
    phase: RoundPhase,
    is_bonus: bool,
    on_click: EventHandler<()>,
}

#[component]
fn Square(props: SquareProps) -> Element {
    let sol_amount = props.deployed as f64 / LAMPORTS_PER_SOL;

    // Calculate percentage of total SOL
    let percentage = if props.total_deployed > 0 {
        (props.deployed as f64 / props.total_deployed as f64 * 100.0) as u32
    } else {
        0
    };

    // Calculate heat intensity (0.0 to 1.0 based on relative deployment)
    let heat_intensity = if props.max_deployed > 0 {
        props.deployed as f64 / props.max_deployed as f64
    } else {
        0.0
    };

    // ORE-style classes
    let base_class = "board-square aspect-square rounded-md flex flex-col p-1.5 cursor-pointer transition-all duration-300 relative overflow-hidden";

    // Phase-aware state classes
    let state_class = match props.phase {
        RoundPhase::Ended => {
            // Round finalized - show winner clearly
            if props.winning {
                "board-square-winner-glow ring-2 ring-gold"
            } else {
                "board-square-loser opacity-40"
            }
        }
        RoundPhase::Revealing => {
            // Reveal phase - show revealed choices and leading square
            if props.leading {
                "board-square-leading ring-2 ring-green-500"
            } else if props.selected {
                "board-square-selected ring-2 ring-blue-500"
            } else {
                ""
            }
        }
        RoundPhase::Committing => {
            // Commit phase: EVERYTHING HIDDEN except user's own selection
            // No leading indicator, no heat map - prevents copying
            if props.selected {
                "board-square-selected ring-2 ring-purple-500"
            } else {
                "board-square-hidden opacity-80"
            }
        }
        RoundPhase::Deploying => {
            // Deploy phase: This shouldn't exist in pure commit-reveal
            // Keep for backwards compatibility but treat like commit
            if props.selected {
                "board-square-selected ring-2 ring-blue-500"
            } else {
                ""
            }
        }
    };

    let opacity_class = if props.disabled && !props.winning && !props.selected && !props.leading {
        "opacity-60 cursor-not-allowed"
    } else {
        ""
    };

    let full_class = format!("{} {} {}", base_class, state_class, opacity_class);

    // Heat map background - ONLY visible during reveal and ended phases
    // Commit phase: NO heat map (prevents copying)
    let heat_bg = match props.phase {
        RoundPhase::Committing | RoundPhase::Deploying => {
            // HIDDEN - uniform purple tint, no heat indication
            "background: rgba(139, 92, 246, 0.1);".to_string()
        }
        RoundPhase::Revealing => {
            // Now visible - gold heat map as reveals come in
            if heat_intensity > 0.0 {
                let alpha = (heat_intensity * 0.4).min(0.4);
                format!("background: linear-gradient(to top, rgba(251, 191, 36, {:.2}) 0%, transparent 100%);", alpha)
            } else {
                String::new()
            }
        }
        RoundPhase::Ended => {
            // Winner highlighted, losers muted
            if props.winning {
                "background: linear-gradient(to top, rgba(251, 191, 36, 0.4) 0%, transparent 100%);".to_string()
            } else {
                String::new()
            }
        }
    };

    rsx! {
        button {
            class: "{full_class}",
            style: "{heat_bg}",
            disabled: props.disabled && !props.winning,
            onclick: move |_| props.on_click.call(()),

            // Top row: indicators - HIDDEN during commit phase
            div { class: "flex justify-between items-start w-full text-xs",
                // Left: Status indicator based on phase
                div { class: "flex items-center gap-0.5",
                    match props.phase {
                        RoundPhase::Committing | RoundPhase::Deploying => {
                            // HIDDEN - only show lock icon, no leading/bonus info
                            rsx! {
                                svg {
                                    class: "w-3 h-3 text-purple-400/50",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "2",
                                    view_box: "0 0 24 24",
                                    path { d: "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" }
                                }
                            }
                        }
                        RoundPhase::Revealing => {
                            if props.leading {
                                // Eye icon during reveal - now visible
                                rsx! {
                                    svg {
                                        class: "w-3 h-3 text-gold",
                                        fill: "none",
                                        stroke: "currentColor",
                                        stroke_width: "2",
                                        view_box: "0 0 24 24",
                                        path { d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                                        path { d: "M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                        RoundPhase::Ended => {
                            if props.winning {
                                // Trophy for winner
                                rsx! {
                                    svg {
                                        class: "w-3.5 h-3.5 text-gold",
                                        fill: "currentColor",
                                        view_box: "0 0 20 20",
                                        path { d: "M5 3a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2V5a2 2 0 00-2-2H5zM5 11a2 2 0 00-2 2v2a2 2 0 002 2h2a2 2 0 002-2v-2a2 2 0 00-2-2H5zM11 5a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V5zM14 11a1 1 0 011 1v1h1a1 1 0 110 2h-1v1a1 1 0 11-2 0v-1h-1a1 1 0 110-2h1v-1a1 1 0 011-1z" }
                                    }
                                }
                            } else {
                                rsx! {}
                            }
                        }
                    }
                }

                // Right: Miner count - ONLY visible during reveal/ended
                if props.count > 0 && (props.phase == RoundPhase::Revealing || props.phase == RoundPhase::Ended) {
                    span { class: "text-low font-mono flex items-center gap-0.5",
                        "{props.count}"
                        svg {
                            class: "w-2.5 h-2.5",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path { d: "M19 14l-7 7m0 0l-7-7m7 7V3" }
                        }
                    }
                }
            }

            // Center: SOL amount + percentage - HIDDEN during commit
            div { class: "flex-1 flex flex-col items-center justify-center",
                match props.phase {
                    RoundPhase::Committing | RoundPhase::Deploying => {
                        // HIDDEN - show "?" for all squares
                        rsx! {
                            span { class: "text-purple-400/70 font-mono text-2xl font-bold", "?" }
                        }
                    }
                    RoundPhase::Revealing => {
                        // Now visible - show SOL amounts as reveals come in
                        rsx! {
                            span { class: "text-high font-mono text-sm font-semibold",
                                {format!("{:.4}", sol_amount)}
                            }
                            if percentage > 0 {
                                span { class: "text-gold font-mono text-xs", "{percentage}%" }
                            }
                        }
                    }
                    RoundPhase::Ended => {
                        // Final state - show everything
                        rsx! {
                            span { class: "text-high font-mono text-sm font-semibold",
                                {format!("{:.4}", sol_amount)}
                            }
                            if props.winning {
                                span { class: "text-gold font-mono text-xs font-bold", "SYNCED!" }
                            } else if percentage > 0 {
                                span { class: "text-low font-mono text-xs", "out of sync" }
                            }
                        }
                    }
                }
            }

            // Bottom: Phase-specific indicator - HIDDEN during commit
            match props.phase {
                RoundPhase::Committing | RoundPhase::Deploying => {
                    // Show square number only (no leading info)
                    rsx! {
                        div { class: "absolute bottom-0.5 left-0 right-0 text-center",
                            span { class: "text-xs text-purple-400/40 font-mono", "#{props.index + 1}" }
                        }
                    }
                }
                RoundPhase::Revealing => {
                    if props.leading {
                        rsx! {
                            div { class: "absolute bottom-0.5 left-0 right-0 text-center",
                                span { class: "text-xs font-medium text-gold", "LEADING" }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }
                RoundPhase::Ended => {
                    if props.winning {
                        rsx! {
                            div { class: "absolute bottom-0.5 left-0 right-0 text-center",
                                span { class: "text-xs font-bold text-gold animate-pulse", "SYNC!" }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }
        }
    }
}
