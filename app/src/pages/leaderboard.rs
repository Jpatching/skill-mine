use dioxus::prelude::*;
use crate::hooks::use_leaderboard;

#[component]
pub fn Leaderboard() -> Element {
    let leaderboard = use_leaderboard();
    let state = leaderboard.read();

    rsx! {
        div { class: "max-w-4xl mx-auto",
            h1 { class: "text-3xl font-bold mb-8", "Skill Leaderboard" }

            div { class: "card",
                if state.loading {
                    div { class: "text-center py-12",
                        div { class: "animate-spin w-8 h-8 border-2 border-skill-400 border-t-transparent rounded-full mx-auto mb-4" }
                        p { class: "text-gray-500", "Loading leaderboard..." }
                    }
                } else if let Some(error) = &state.error {
                    div { class: "text-center py-12",
                        p { class: "text-red-400", "Error: {error}" }
                    }
                } else if state.entries.is_empty() {
                    div { class: "text-center py-12",
                        p { class: "text-gray-500", "No miners with skill activity yet. Be the first!" }
                    }
                } else {
                    // Header
                    div { class: "grid grid-cols-5 gap-4 pb-3 border-b border-gray-700 text-sm text-gray-500",
                        div { "Rank" }
                        div { class: "col-span-2", "Address" }
                        div { class: "text-right", "Score" }
                        div { class: "text-right", "Win Rate" }
                    }

                    // Entries
                    div { class: "divide-y divide-gray-800",
                        for entry in state.entries.iter() {
                            div { class: "grid grid-cols-5 gap-4 py-3 items-center",
                                // Rank
                                div {
                                    if entry.rank <= 3 {
                                        span { class: "text-2xl",
                                            match entry.rank {
                                                1 => "🥇",
                                                2 => "🥈",
                                                3 => "🥉",
                                                _ => "",
                                            }
                                        }
                                    } else {
                                        span { class: "text-gray-400 font-mono", "#{entry.rank}" }
                                    }
                                }

                                // Address
                                div { class: "col-span-2 font-mono text-sm",
                                    {
                                        let addr = &entry.address;
                                        let short = format!("{}...{}", &addr[..8], &addr[addr.len()-8..]);
                                        let url = format!("https://explorer.solana.com/address/{}?cluster=devnet", addr);
                                        rsx! {
                                            a {
                                                href: "{url}",
                                                target: "_blank",
                                                class: "text-gray-300 hover:text-skill-400 transition-colors",
                                                "{short}"
                                            }
                                        }
                                    }
                                }

                                // Score
                                div { class: "text-right",
                                    span { class: "font-mono text-skill-400", "{entry.skill_score}" }
                                    if entry.streak > 0 {
                                        span { class: "ml-2 text-xs text-yellow-400", "🔥{entry.streak}" }
                                    }
                                }

                                // Win rate
                                div { class: "text-right font-mono text-gray-400",
                                    "{entry.win_rate:.1}%"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
