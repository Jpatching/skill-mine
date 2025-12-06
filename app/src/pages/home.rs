use dioxus::prelude::*;
use crate::route::Route;

#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "max-w-4xl mx-auto text-center py-16",
            // Hero
            h1 { class: "text-5xl font-bold mb-6",
                span { class: "text-skill-400", "SKILL" }
                span { class: "text-gray-100", " Mining" }
            }

            p { class: "text-xl text-gray-400 mb-8 max-w-2xl mx-auto",
                "Skill-based mining on Solana. Predict the winning square, "
                "build your streak, and earn up to 1.5x multiplier on rewards."
            }

            // CTA buttons
            div { class: "flex justify-center gap-4 mb-16",
                Link {
                    to: Route::Play {},
                    class: "btn btn-primary text-lg px-8 py-3",
                    "Start Playing"
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
                    title: "Predict",
                    description: "Choose which of the 25 squares will win each round.",
                    icon: "🎯",
                }
                FeatureCard {
                    title: "Build Streak",
                    description: "Consecutive correct predictions increase your multiplier.",
                    icon: "🔥",
                }
                FeatureCard {
                    title: "Earn More",
                    description: "Up to 1.5x reward multiplier based on your skill score.",
                    icon: "💰",
                }
            }

            // Multiplier breakdown
            div { class: "mt-16 card max-w-xl mx-auto",
                h3 { class: "text-xl font-semibold text-skill-400 mb-4", "Skill Multiplier Formula" }
                div { class: "text-left space-y-2 text-gray-300",
                    p {
                        span { class: "text-gray-500", "Base: " }
                        "1.00x"
                    }
                    p {
                        span { class: "text-gray-500", "Score Bonus: " }
                        "+5% per order of magnitude"
                    }
                    p {
                        span { class: "text-gray-500", "Streak Bonus: " }
                        "+2% per consecutive win (max 10)"
                    }
                    p {
                        span { class: "text-gray-500", "Maximum: " }
                        span { class: "text-skill-400 font-semibold", "1.50x" }
                    }
                }
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
