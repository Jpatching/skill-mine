use dioxus::prelude::*;
use crate::components::SkillStats;
use crate::hooks::use_miner;
use crate::{WalletState, MinerState};

#[component]
pub fn Stats() -> Element {
    let wallet = use_context::<Signal<WalletState>>();
    let miner = use_miner();

    let wallet_read = wallet.read();
    let miner_read = miner.read();

    rsx! {
        div { class: "max-w-4xl mx-auto",
            h1 { class: "text-3xl font-bold mb-8", "Your Stats" }

            if !wallet_read.connected {
                div { class: "card text-center py-12",
                    p { class: "text-gray-500 mb-4", "Connect your wallet to view your stats" }
                }
            } else {
                div { class: "grid md:grid-cols-2 gap-6",
                    // Skill stats card
                    SkillStats {}

                    // Detailed stats
                    div { class: "card",
                        h3 { class: "text-lg font-semibold text-skill-400 mb-4", "Mining Stats" }
                        div { class: "space-y-3",
                            DetailRow {
                                label: "Wallet",
                                value: wallet_read.pubkey.clone().unwrap_or_default(),
                                truncate: true,
                            }
                            DetailRow {
                                label: "Pending SOL",
                                value: format!("{:.6} SOL", miner_read.rewards_sol as f64 / 1_000_000_000.0),
                                truncate: false,
                            }
                            DetailRow {
                                label: "Pending SKILL",
                                value: format!("{:.6} SKILL", miner_read.rewards_ore as f64 / 100_000_000_000.0),
                                truncate: false,
                            }
                        }
                    }
                }

                // Prediction history (placeholder)
                div { class: "card mt-6",
                    h3 { class: "text-lg font-semibold text-skill-400 mb-4", "Recent Predictions" }
                    p { class: "text-gray-500 text-center py-8",
                        "Prediction history coming soon..."
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DetailRowProps {
    label: &'static str,
    value: String,
    truncate: bool,
}

#[component]
fn DetailRow(props: DetailRowProps) -> Element {
    let display_value = if props.truncate && props.value.len() > 16 {
        format!("{}...{}", &props.value[..8], &props.value[props.value.len()-8..])
    } else {
        props.value.clone()
    };

    rsx! {
        div { class: "flex justify-between items-center",
            span { class: "text-gray-500", "{props.label}" }
            span { class: "font-mono text-gray-300", "{display_value}" }
        }
    }
}
