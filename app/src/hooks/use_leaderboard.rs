use base64::Engine;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{HELIUS_API_KEY, PROGRAM_ID};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub address: String,
    pub skill_score: u64,
    pub streak: u16,
    pub win_rate: f64,
}

#[derive(Clone, Default)]
pub struct LeaderboardState {
    pub entries: Vec<LeaderboardEntry>,
    pub loading: bool,
    pub error: Option<String>,
}

pub fn use_leaderboard() -> Signal<LeaderboardState> {
    let mut state = use_signal(LeaderboardState::default);

    // Use use_resource instead of use_effect + spawn for safer async
    let _resource = use_resource(move || {
        async move {
            match fetch_leaderboard().await {
                Ok(entries) => {
                    let mut s = state.write();
                    s.entries = entries;
                    s.loading = false;
                }
                Err(e) => {
                    let mut s = state.write();
                    s.error = Some(e);
                    s.loading = false;
                }
            }
        }
    });

    state
}

#[derive(Serialize)]
struct HeliusRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: &'static str,
    params: HeliusParams,
}

#[derive(Serialize)]
struct HeliusParams {
    #[serde(rename = "programId")]
    program_id: String,
    encoding: &'static str,
    filters: Vec<HeliusFilter>,
}

#[derive(Serialize)]
struct HeliusFilter {
    dataSize: usize,
}

#[derive(Deserialize)]
struct HeliusResponse {
    result: Option<Vec<HeliusAccount>>,
}

#[derive(Deserialize)]
struct HeliusAccount {
    pubkey: String,
    account: HeliusAccountData,
}

#[derive(Deserialize)]
struct HeliusAccountData {
    data: (String, String),
}

async fn fetch_leaderboard() -> Result<Vec<LeaderboardEntry>, String> {
    let client = reqwest::Client::new();
    let url = format!("https://devnet.helius-rpc.com/?api-key={}", HELIUS_API_KEY);

    // Miner account size: 8 (discriminator) + 536 bytes
    let miner_size = 544;

    let request = HeliusRequest {
        jsonrpc: "2.0",
        id: "skill-leaderboard",
        method: "getProgramAccounts",
        params: HeliusParams {
            program_id: PROGRAM_ID.to_string(),
            encoding: "base64",
            filters: vec![HeliusFilter { dataSize: miner_size }],
        },
    };

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let helius_response: HeliusResponse = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let mut entries: Vec<LeaderboardEntry> = vec![];

    if let Some(accounts) = helius_response.result {
        for account in accounts {
            let data = base64::engine::general_purpose::STANDARD
                .decode(&account.account.data.0)
                .unwrap_or_default();

            if data.len() >= 536 {
                // Check discriminator (Miner = specific value)
                // Parse skill fields
                let skill_score = u64::from_le_bytes(data[496..504].try_into().unwrap_or_default());
                let streak = u16::from_le_bytes(data[506..508].try_into().unwrap_or_default());
                let challenge_count = u64::from_le_bytes(data[520..528].try_into().unwrap_or_default());
                let challenge_wins = u64::from_le_bytes(data[528..536].try_into().unwrap_or_default());

                let win_rate = if challenge_count > 0 {
                    (challenge_wins as f64 / challenge_count as f64) * 100.0
                } else {
                    0.0
                };

                // Only include miners with skill activity
                if skill_score > 0 || challenge_count > 0 {
                    entries.push(LeaderboardEntry {
                        rank: 0,
                        address: account.pubkey,
                        skill_score,
                        streak,
                        win_rate,
                    });
                }
            }
        }
    }

    // Sort by skill score descending
    entries.sort_by(|a, b| b.skill_score.cmp(&a.skill_score));

    // Assign ranks
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.rank = i + 1;
    }

    // Return top 100
    Ok(entries.into_iter().take(100).collect())
}
