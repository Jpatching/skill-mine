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
}

#[component]
pub fn Board(props: BoardProps) -> Element {
    rsx! {
        div { class: "space-y-3",
            // 5x5 Grid - ORE style
            div { class: "grid grid-cols-5 gap-1.5",
                for i in 0..25u8 {
                    Square {
                        index: i,
                        selected: props.selected.contains(&i),
                        winning: props.winning_square == Some(i),
                        deployed: props.deployed[i as usize],
                        count: props.count[i as usize],
                        disabled: props.disabled,
                        phase: props.phase,
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
    deployed: u64,
    count: u64,
    disabled: bool,
    phase: RoundPhase,
    on_click: EventHandler<()>,
}

#[component]
fn Square(props: SquareProps) -> Element {
    let sol_amount = props.deployed as f64 / LAMPORTS_PER_SOL;
    let display_num = props.index + 1; // Display as 1-25 instead of 0-24

    // ORE-style classes
    let base_class = "board-square aspect-square rounded-md flex flex-col p-2 cursor-pointer transition-all duration-300 relative";

    // Phase-aware state classes
    let state_class = match props.phase {
        RoundPhase::Revealing | RoundPhase::Ended => {
            if props.winning {
                "board-square-winner-glow" // Winner with glow animation
            } else {
                "board-square-loser" // Fade out non-winners
            }
        }
        RoundPhase::Deploying => {
            if props.winning {
                "board-square-winning"
            } else if props.selected {
                "board-square-selected"
            } else {
                ""
            }
        }
    };

    let opacity_class = if props.disabled && !props.winning && !props.selected {
        "opacity-50 cursor-not-allowed"
    } else {
        ""
    };

    let full_class = format!("{} {} {}", base_class, state_class, opacity_class);

    rsx! {
        button {
            class: "{full_class}",
            disabled: props.disabled && !props.winning,
            onclick: move |_| props.on_click.call(()),

            // Top row: square number (left) + miner count (right)
            div { class: "flex justify-between items-start w-full text-xs",
                // Square number
                span { class: "text-low font-mono", "#{display_num}" }
                // Miner count with arrow
                if props.count > 0 {
                    span { class: "text-low font-mono flex items-center gap-0.5",
                        "{props.count}"
                        // Down arrow icon
                        svg {
                            class: "w-3 h-3",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            view_box: "0 0 24 24",
                            path {
                                d: "M19 14l-7 7m0 0l-7-7m7 7V3"
                            }
                        }
                    }
                }
            }

            // Center: SOL amount
            div { class: "flex-1 flex items-center justify-center",
                span { class: "text-high font-mono text-sm",
                    {format!("{:.4}", sol_amount)}
                }
            }

            // Winner indicator (bottom)
            if props.winning {
                div { class: "absolute bottom-1 left-0 right-0 text-center",
                    span { class: "text-xs font-bold text-gold", "WIN" }
                }
            }
        }
    }
}
