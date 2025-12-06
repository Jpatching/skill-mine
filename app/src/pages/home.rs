use dioxus::prelude::*;
use crate::route::Route;

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "max-w-4xl mx-auto text-center py-16",
            // Hero
            h1 { class: "text-5xl font-bold mb-4",
                span { class: "text-skill-400", "SYNC" }
            }
            p { class: "text-2xl text-gray-300 mb-2", "Social Mining on Solana" }
            p { class: "text-lg text-gray-500 mb-8", "Mine together. Win together." }

            p { class: "text-xl text-gray-400 mb-8 max-w-2xl mx-auto",
                "Pick a square. If the majority picks the same one, you all win. "
                "No hardware. No hashrate. Just you and the crowd."
            }

            // CTA buttons
            div { class: "flex justify-center gap-4 mb-16",
                Link {
                    to: Route::Play {},
                    class: "btn btn-primary text-lg px-8 py-3",
                    "Join the Round"
                }
                Link {
                    to: Route::Leaderboard {},
                    class: "btn btn-secondary text-lg px-8 py-3",
                    "View Leaderboard"
                }
            }

            // How it works
            div { class: "grid md:grid-cols-3 gap-8 mt-16",
                FeatureCard {
                    title: "Pick",
                    description: "Where's the crowd going? Choose your square.",
                    icon: "🎯",
                }
                FeatureCard {
                    title: "Sync",
                    description: "If you pick what others pick, you all share the pot.",
                    icon: "🤝",
                }
                FeatureCard {
                    title: "Streak",
                    description: "Build your sync streak for bonus multipliers.",
                    icon: "🔥",
                }
            }

            // The vibe
            div { class: "mt-16 card max-w-xl mx-auto",
                h3 { class: "text-xl font-semibold text-skill-400 mb-4", "The Vibe" }
                div { class: "text-left space-y-3 text-gray-300",
                    p { class: "text-lg",
                        "Not: "
                        span { class: "text-gray-500 line-through", "\"I'm skilled, I beat you\"" }
                    }
                    p { class: "text-lg",
                        "Yes: "
                        span { class: "text-skill-400 font-semibold", "\"We synced, we all eat\"" }
                    }
                }
            }

            // Tagline
            div { class: "mt-16",
                p { class: "text-2xl font-bold text-gray-500", "Sync or sink." }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct FeatureCardProps {
    title: &'static str,
    description: &'static str,
    icon: &'static str,
}

#[component]
fn FeatureCard(props: FeatureCardProps) -> Element {
    rsx! {
        div { class: "card text-center",
            div { class: "text-4xl mb-4", "{props.icon}" }
            h3 { class: "text-lg font-semibold text-skill-400 mb-2", "{props.title}" }
            p { class: "text-gray-400", "{props.description}" }
        }
    }
}
