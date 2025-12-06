use dioxus::prelude::*;
use crate::route::Route;
use crate::components::WalletButton;

#[component]
pub fn Layout() -> Element {
    rsx! {
        div { class: "min-h-screen",
            style: "background-color: var(--surface-base);",
            // Navigation
            nav { class: "border-b elevated-border backdrop-blur sticky top-0 z-50",
                style: "background-color: var(--surface-base);",
                div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div { class: "flex justify-between h-16",
                        // Logo - links to game
                        div { class: "flex items-center",
                            Link { to: Route::Play {}, class: "flex items-center space-x-2",
                                span { class: "text-2xl font-bold text-skill-400", "SKILL" }
                            }
                        }

                        // Nav links
                        div { class: "hidden sm:flex sm:items-center sm:space-x-8",
                            NavLink { to: Route::Play {}, label: "Game" }
                            NavLink { to: Route::Leaderboard {}, label: "Leaderboard" }
                            NavLink { to: Route::Stats {}, label: "Stats" }
                            NavLink { to: Route::Home {}, label: "About" }
                        }

                        // Wallet button
                        div { class: "flex items-center",
                            WalletButton {}
                        }
                    }
                }
            }

            // Main content
            main { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8",
                Outlet::<Route> {}
            }

            // Footer
            footer { class: "border-t elevated-border py-8 mt-auto",
                div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 text-center text-low",
                    p { "SKILL - Skill-Based Mining on Solana" }
                    p { class: "text-sm mt-2",
                        "Program: "
                        code { class: "text-gold", "{crate::PROGRAM_ID}" }
                    }
                }
            }
        }
    }
}

#[component]
fn NavLink(to: Route, label: &'static str) -> Element {
    rsx! {
        Link {
            to: to,
            class: "text-mid hover:text-gold px-3 py-2 text-sm font-medium transition-colors",
            "{label}"
        }
    }
}
