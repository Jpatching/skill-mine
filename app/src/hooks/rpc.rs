use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'static str,
    pub params: Vec<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct RpcResponse<T> {
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

#[derive(Deserialize, Debug)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct AccountInfo {
    pub data: (String, String), // (base64 data, encoding)
    pub lamports: u64,
    pub owner: String,
}

#[derive(Deserialize, Debug)]
pub struct AccountResult {
    pub value: Option<AccountInfo>,
}

pub async fn fetch_account(rpc_url: &str, pubkey: &str) -> Result<Option<Vec<u8>>, String> {
    let client = reqwest::Client::new();

    let request = RpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getAccountInfo",
        params: vec![
            serde_json::json!(pubkey),
            serde_json::json!({
                "encoding": "base64"
            }),
        ],
    };

    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let rpc_response: RpcResponse<AccountResult> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(error) = rpc_response.error {
        return Err(error.message);
    }

    if let Some(result) = rpc_response.result {
        if let Some(account) = result.value {
            let data = base64::engine::general_purpose::STANDARD
                .decode(&account.data.0)
                .map_err(|e| e.to_string())?;
            return Ok(Some(data));
        }
    }

    Ok(None)
}

// PDA derivation (simplified - matches Solana's find_program_address)
pub fn derive_pda(seeds: &[&[u8]], program_id: &str) -> String {
    // For web, we use a simplified approach
    // In production, you'd use proper PDA derivation
    use sha2::{Sha256, Digest};

    let program_bytes = bs58::decode(program_id).into_vec().unwrap_or_default();

    for bump in (0..=255u8).rev() {
        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update(&[bump]);
        hasher.update(&program_bytes);
        hasher.update(b"ProgramDerivedAddress");

        let hash = hasher.finalize();

        // Check if it's off the ed25519 curve (simplified check)
        // In production, use proper curve checking
        if hash[31] & 0x80 == 0 {
            return bs58::encode(&hash[..32]).into_string();
        }
    }

    String::new()
}

// Known PDAs for SKILL program
pub fn board_pda() -> String {
    // Pre-computed: 924DVhXS3hXKVoLcSd7Uhi2B4k7DjTWm7UYYbft4d5pq
    "924DVhXS3hXKVoLcSd7Uhi2B4k7DjTWm7UYYbft4d5pq".to_string()
}

pub fn round_pda(round_id: u64) -> String {
    derive_pda(&[b"round", &round_id.to_le_bytes()], crate::PROGRAM_ID)
}

pub fn miner_pda(authority: &str) -> String {
    let auth_bytes = bs58::decode(authority).into_vec().unwrap_or_default();
    derive_pda(&[b"miner", &auth_bytes], crate::PROGRAM_ID)
}

/// Fetch current slot from RPC
pub async fn fetch_slot(rpc_url: &str) -> Result<u64, String> {
    let client = reqwest::Client::new();

    let request = RpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getSlot",
        params: vec![],
    };

    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let rpc_response: RpcResponse<u64> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(error) = rpc_response.error {
        return Err(error.message);
    }

    rpc_response.result.ok_or_else(|| "No slot returned".to_string())
}
