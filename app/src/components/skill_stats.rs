use dioxus::prelude::*;
use crate::MinerState;

#[component]
pub fn SkillStats() -> Element {
    let miner = use_context::<Signal<MinerState>>();
    let miner_read = miner.read();

    // Calculate skill multiplier (matching api/src/state/miner.rs logic)
    let multiplier = calculate_multiplier(miner_read.skill_score, miner_read.streak);
    let win_rate = if miner_read.challenge_count > 0 {
        (miner_read.challenge_wins as f64 / miner_read.challenge_count as f64) * 100.0
    } else {
        0.0
    };

    rsx! {
        div { class: "card",
            h3 { class: "text-lg font-semibold text-skill-400 mb-4", "Skill Stats" }

            if miner_read.loading {
                div { class: "animate-pulse space-y-3",
                    div { class: "h-4 bg-gray-700 rounded w-3/4" }
                    div { class: "h-4 bg-gray-700 rounded w-1/2" }
                    div { class: "h-4 bg-gray-700 rounded w-2/3" }
                }
            } else {
                div { class: "space-y-3",
                    StatRow {
                        label: "Skill Score",
                        value: format!("{}", miner_read.skill_score),
                    }
                    StatRow {
                        label: "Streak",
                        value: format!("{} wins", miner_read.streak),
                        highlight: miner_read.streak > 0,
                    }
                    StatRow {
                        label: "Multiplier",
                        value: format!("{:.2}x", multiplier as f64 / 100.0),
                        highlight: multiplier > 100,
                    }
                    StatRow {
                        label: "Win Rate",
                        value: format!("{:.1}%", win_rate),
                    }
                    StatRow {
                        label: "Predictions",
                        value: format!("{}/{}", miner_read.challenge_wins, miner_read.challenge_count),
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct StatRowProps {
    label: &'static str,
    value: String,
    #[props(default = false)]
    highlight: bool,
}

#[component]
fn StatRow(props: StatRowProps) -> Element {
    let value_class = if props.highlight {
        "text-skill-400 font-semibold"
    } else {
        "text-gray-300"
    };

    rsx! {
        div { class: "flex justify-between items-center",
            span { class: "text-gray-500", "{props.label}" }
            span { class: "{value_class} font-mono", "{props.value}" }
        }
    }
}

// Multiplier calculation (matches api/src/state/miner.rs)
fn calculate_multiplier(skill_score: u64, streak: u16) -> u64 {
    let base = 100u64;

    // Score bonus: +5% per order of magnitude
    let score_bonus = if skill_score > 0 {
        let log_approx = (64 - skill_score.leading_zeros()) as u64 * 3 / 10;
        log_approx.saturating_mul(5)
    } else {
        0
    };

    // Streak bonus: +2% per consecutive win, max 10
    let streak_bonus = (streak.min(10) as u64).saturating_mul(2);

    // Total capped at 150 (1.50x)
    (base + score_bonus + streak_bonus).min(150)
}
