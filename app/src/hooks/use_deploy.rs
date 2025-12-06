use base64::Engine;
use wasm_bindgen::prelude::*;
use js_sys::{Object, Reflect, Promise, Uint8Array, Array};

use crate::RPC_URL;
use super::rpc::{board_pda, round_pda, miner_pda, derive_pda, fetch_account, RpcRequest, RpcResponse};

// Program IDs
pub const PROGRAM_ID: &str = "3vzFzHFytiu7zkctgwX2JJhXq3XdN8J7U2WFongrejoU";
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const SLOT_HASHES_SYSVAR: &str = "SysvarS1otHashes111111111111111111111111111";

// Instruction discriminators (from api/src/instruction.rs)
const CHECKPOINT_DISCRIMINATOR: u8 = 2;
const DEPLOY_DISCRIMINATOR: u8 = 6;
const RESET_DISCRIMINATOR: u8 = 14;

// Constants matching program
const INTERMISSION_SLOTS: u64 = 35;

/// Build deploy transaction and send via Phantom
/// Schelling Point: No entropy accounts needed
#[cfg(feature = "web")]
pub async fn deploy_transaction(
    authority: &str,
    amount_lamports: u64,
    selected_squares: &[u8],
    round_id: u64,
) -> Result<String, String> {
    // 1. Calculate all PDAs
    let board = board_pda();
    let round = round_pda(round_id);
    let miner = miner_pda(authority);
    let automation = automation_pda(authority);

    // 2. Build squares bitmask
    let squares_mask: u32 = selected_squares.iter().fold(0u32, |acc, &sq| acc | (1 << sq));

    // 3. Build instruction data
    // [discriminator (1 byte)] [amount (8 bytes)] [squares (4 bytes)]
    let mut ix_data = vec![DEPLOY_DISCRIMINATOR];
    ix_data.extend_from_slice(&amount_lamports.to_le_bytes());
    ix_data.extend_from_slice(&squares_mask.to_le_bytes());

    // 4. Get recent blockhash
    let blockhash = fetch_recent_blockhash(RPC_URL).await?;

    // 5. Build and send transaction via Phantom using JS interop
    send_deploy_tx_phantom(
        authority,
        &board,
        &miner,
        &round,
        &automation,
        &ix_data,
        &blockhash,
    ).await
}

fn automation_pda(authority: &str) -> String {
    let auth_bytes = bs58::decode(authority).into_vec().unwrap_or_default();
    derive_pda(&[b"automation", &auth_bytes], PROGRAM_ID)
}

async fn fetch_recent_blockhash(rpc_url: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let request = RpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getLatestBlockhash",
        params: vec![],
    };

    let response = client
        .post(rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    #[derive(serde::Deserialize)]
    struct BlockhashResult {
        value: BlockhashValue,
    }

    #[derive(serde::Deserialize)]
    struct BlockhashValue {
        blockhash: String,
    }

    let rpc_response: RpcResponse<BlockhashResult> = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(error) = rpc_response.error {
        return Err(error.message);
    }

    rpc_response.result
        .map(|r| r.value.blockhash)
        .ok_or_else(|| "No blockhash returned".to_string())
}

/// Send transaction via Phantom using JavaScript interop
/// Schelling Point: No entropy accounts needed
#[cfg(feature = "web")]
async fn send_deploy_tx_phantom(
    authority: &str,
    board: &str,
    miner: &str,
    round: &str,
    automation: &str,
    ix_data: &[u8],
    blockhash: &str,
) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;

    // Check for Phantom
    let solana = Reflect::get(&window, &JsValue::from_str("solana"))
        .map_err(|_| "Phantom not found")?;

    if solana.is_undefined() {
        return Err("Phantom wallet not connected".to_string());
    }

    // Build transaction using custom JS
    let result = build_and_send_tx_js(
        &solana,
        authority,
        board,
        miner,
        round,
        automation,
        ix_data,
        blockhash,
    ).await?;

    Ok(result)
}

/// Build and send transaction via JavaScript
/// Schelling Point: No entropy accounts needed
#[cfg(feature = "web")]
async fn build_and_send_tx_js(
    solana: &JsValue,
    authority: &str,
    board: &str,
    miner: &str,
    round: &str,
    automation: &str,
    ix_data: &[u8],
    blockhash: &str,
) -> Result<String, String> {
    // We need to serialize a proper Solana transaction
    // Format: [signature_count][...signatures][message]
    // Message: [header][accounts][blockhash][instructions]

    // Accounts in order (matching sdk.rs deploy function):
    // 0: signer (writable, signer)
    // 1: authority (writable) - same as signer for user deploy
    // 2: automation (writable)
    // 3: board (writable)
    // 4: miner (writable)
    // 5: round (writable)
    // 6: system_program (readonly)

    let accounts = vec![
        (authority, true, true),      // signer, writable
        (authority, true, false),     // authority, writable (same as signer)
        (automation, true, false),    // automation, writable
        (board, true, false),         // board, writable
        (miner, true, false),         // miner, writable
        (round, true, false),         // round, writable
        (SYSTEM_PROGRAM, false, false), // system_program, readonly
    ];

    // Build serialized transaction message
    let tx_bytes = build_transaction_bytes(
        authority,
        &accounts,
        PROGRAM_ID,
        ix_data,
        blockhash,
    )?;

    // Convert to Uint8Array
    let tx_array = Uint8Array::new_with_length(tx_bytes.len() as u32);
    tx_array.copy_from(&tx_bytes);

    // Call Phantom's signAndSendTransaction
    let sign_fn = Reflect::get(solana, &JsValue::from_str("signAndSendTransaction"))
        .map_err(|_| "No signAndSendTransaction method")?;

    let sign_fn: js_sys::Function = sign_fn.dyn_into()
        .map_err(|_| "signAndSendTransaction is not a function")?;

    // Phantom accepts { message: Uint8Array } for legacy transactions
    // or the raw VersionedTransaction
    let tx_obj = Object::new();
    // Try passing the serialized bytes directly
    Reflect::set(&tx_obj, &JsValue::from_str("serialize"), &tx_array.clone().into())
        .map_err(|_| "Failed to set serialize")?;

    // Some versions of Phantom want the message
    let promise = sign_fn.call1(solana, &tx_array.into())
        .map_err(|e| format!("Sign call failed: {:?}", e))?;

    let promise: Promise = promise.dyn_into()
        .map_err(|_| "Not a promise")?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Transaction rejected: {:?}", e))?;

    // Get signature from result
    let signature = Reflect::get(&result, &JsValue::from_str("signature"))
        .ok()
        .and_then(|s| s.as_string())
        .ok_or("No signature in response")?;

    Ok(signature)
}

/// Build a legacy Solana transaction as raw bytes
/// Returns the unsigned transaction message (Phantom will sign)
fn build_transaction_bytes(
    fee_payer: &str,
    accounts: &[(&str, bool, bool)], // (pubkey, writable, signer)
    program_id: &str,
    ix_data: &[u8],
    blockhash: &str,
) -> Result<Vec<u8>, String> {
    // Legacy transaction format:
    // Message header: [num_required_signatures, num_readonly_signed, num_readonly_unsigned]
    // Account addresses: [compact-u16 count][...32-byte pubkeys]
    // Recent blockhash: [32 bytes]
    // Instructions: [compact-u16 count][...instructions]
    // Each instruction: [program_id_index][compact-u16 account_count][...account_indices][compact-u16 data_len][...data]

    // Deduplicate accounts and build lookup
    let mut unique_accounts: Vec<String> = Vec::new();
    let mut account_metas: Vec<(usize, bool, bool)> = Vec::new(); // (index, writable, signer)

    // Fee payer is always first and signer
    unique_accounts.push(fee_payer.to_string());

    for (pubkey, writable, signer) in accounts {
        if let Some(idx) = unique_accounts.iter().position(|a| a == *pubkey) {
            account_metas.push((idx, *writable, *signer));
        } else {
            account_metas.push((unique_accounts.len(), *writable, *signer));
            unique_accounts.push(pubkey.to_string());
        }
    }

    // Add program ID
    let program_idx = if let Some(idx) = unique_accounts.iter().position(|a| a == program_id) {
        idx
    } else {
        let idx = unique_accounts.len();
        unique_accounts.push(program_id.to_string());
        idx
    };

    // Calculate header
    let num_signers = 1u8; // Only the fee payer/authority signs
    let num_readonly_signed = 0u8;
    let num_readonly_unsigned = unique_accounts.iter()
        .enumerate()
        .filter(|(i, _)| {
            // Count readonly unsigned accounts
            *i > 0 && !account_metas.iter().any(|(idx, w, s)| *idx == *i && (*w || *s))
        })
        .count() as u8;

    let mut message = Vec::new();

    // Header
    message.push(num_signers);
    message.push(num_readonly_signed);
    message.push(num_readonly_unsigned);

    // Account addresses (compact array)
    message.extend(compact_u16(unique_accounts.len() as u16));
    for account in &unique_accounts {
        let bytes = bs58::decode(account).into_vec().map_err(|e| e.to_string())?;
        if bytes.len() != 32 {
            return Err(format!("Invalid pubkey length: {} for {}", bytes.len(), account));
        }
        message.extend(&bytes);
    }

    // Recent blockhash
    let blockhash_bytes = bs58::decode(blockhash).into_vec().map_err(|e| e.to_string())?;
    if blockhash_bytes.len() != 32 {
        return Err("Invalid blockhash length".to_string());
    }
    message.extend(&blockhash_bytes);

    // Instructions (1 instruction)
    message.extend(compact_u16(1)); // instruction count

    // Instruction: program_id_index
    message.push(program_idx as u8);

    // Instruction: account indices
    let ix_account_indices: Vec<u8> = account_metas.iter()
        .map(|(idx, _, _)| *idx as u8)
        .collect();
    message.extend(compact_u16(ix_account_indices.len() as u16));
    message.extend(&ix_account_indices);

    // Instruction: data
    message.extend(compact_u16(ix_data.len() as u16));
    message.extend(ix_data);

    // For unsigned transaction, prepend empty signature count
    let mut tx = Vec::new();
    tx.push(0u8); // 0 signatures (wallet will add)
    tx.extend(&message);

    Ok(tx)
}

/// Encode u16 as Solana compact-u16 format
fn compact_u16(val: u16) -> Vec<u8> {
    if val < 0x80 {
        vec![val as u8]
    } else if val < 0x4000 {
        vec![(val & 0x7f) as u8 | 0x80, (val >> 7) as u8]
    } else {
        vec![(val & 0x7f) as u8 | 0x80, ((val >> 7) & 0x7f) as u8 | 0x80, (val >> 14) as u8]
    }
}

#[cfg(not(feature = "web"))]
pub async fn deploy_transaction(
    _authority: &str,
    _amount_lamports: u64,
    _selected_squares: &[u8],
    _round_id: u64,
) -> Result<String, String> {
    Err("Deploy only available in web mode".to_string())
}

// ============ PLAY TRANSACTION (v0.5 - Auto Reset) ============

/// Pre-computed PDAs for SKILL protocol
fn config_pda() -> String {
    // Pre-computed: J1MkbQ4Yu4zHhcj3B34XHfcqufpBpyjQoAxYwy1KsAXj
    "J1MkbQ4Yu4zHhcj3B34XHfcqufpBpyjQoAxYwy1KsAXj".to_string()
}

fn treasury_pda() -> String {
    // Pre-computed: 75mND1dHyZcXntj2m4iFdT9ZwwDTbFCMjDDNQdyz2t2c
    "75mND1dHyZcXntj2m4iFdT9ZwwDTbFCMjDDNQdyz2t2c".to_string()
}

fn mint_pda() -> String {
    // Pre-computed: BAeSqDykZ4SUrHChTFXnWV1vazWMMwi3iDMA5okhF8eB
    "BAeSqDykZ4SUrHChTFXnWV1vazWMMwi3iDMA5okhF8eB".to_string()
}

fn treasury_tokens_pda() -> String {
    // Pre-computed: FyDJZfkXcL6LWfS8dZyvUQAUrTp44ewNYXA3R69bwR4q
    "FyDJZfkXcL6LWfS8dZyvUQAUrTp44ewNYXA3R69bwR4q".to_string()
}

/// Check if round has ended and needs reset
pub async fn check_round_needs_reset() -> Result<(bool, u64, u64, String), String> {
    // Fetch board state
    let board_bytes = fetch_account(RPC_URL, &board_pda()).await?
        .ok_or("Board account not found")?;

    let round_id = u64::from_le_bytes(board_bytes[8..16].try_into().unwrap_or_default());
    let end_slot = u64::from_le_bytes(board_bytes[24..32].try_into().unwrap_or_default());

    // Fetch current slot
    let client = reqwest::Client::new();
    let request = RpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getSlot",
        params: vec![],
    };
    let response = client.post(RPC_URL).json(&request).send().await.map_err(|e| e.to_string())?;
    let rpc_response: RpcResponse<u64> = response.json().await.map_err(|e| e.to_string())?;
    let current_slot = rpc_response.result.ok_or("No slot")?;

    // Fetch config for fee_collector
    let config_bytes = fetch_account(RPC_URL, &config_pda()).await?
        .ok_or("Config account not found")?;
    // Config layout: discriminator (8) + admin (32) + fee_collector (32)
    let fee_collector_bytes = &config_bytes[40..72];
    let fee_collector = bs58::encode(fee_collector_bytes).into_string();

    // Check if round ended
    let round_ended = end_slot != u64::MAX && current_slot >= end_slot + INTERMISSION_SLOTS;

    Ok((round_ended, round_id, current_slot, fee_collector))
}

/// Play transaction - automatically handles reset if round ended
/// This is the main entry point for players in v0.5
#[cfg(feature = "web")]
pub async fn play_transaction(
    authority: &str,
    amount_lamports: u64,
    selected_squares: &[u8],
) -> Result<String, String> {
    // Check if reset is needed
    let (round_ended, round_id, _current_slot, fee_collector) = check_round_needs_reset().await?;

    // Get blockhash
    let blockhash = fetch_recent_blockhash(RPC_URL).await?;

    if round_ended {
        // Bundle reset + deploy in one transaction
        tracing::info!("Round {} ended - bundling reset + deploy", round_id);
        send_play_tx_with_reset(
            authority,
            &fee_collector,
            round_id,
            amount_lamports,
            selected_squares,
            &blockhash,
        ).await
    } else {
        // Just deploy
        tracing::info!("Round {} active - deploying", round_id);
        deploy_transaction(authority, amount_lamports, selected_squares, round_id).await
    }
}

/// Build and send transaction with reset + deploy
#[cfg(feature = "web")]
async fn send_play_tx_with_reset(
    authority: &str,
    fee_collector: &str,
    round_id: u64,
    amount_lamports: u64,
    selected_squares: &[u8],
    blockhash: &str,
) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;

    let solana = Reflect::get(&window, &JsValue::from_str("solana"))
        .map_err(|_| "Phantom not found")?;

    if solana.is_undefined() {
        return Err("Phantom wallet not connected".to_string());
    }

    // Build transaction with two instructions: reset + deploy
    let tx_bytes = build_play_transaction_bytes(
        authority,
        fee_collector,
        round_id,
        amount_lamports,
        selected_squares,
        blockhash,
    )?;

    // Convert to Uint8Array
    let tx_array = Uint8Array::new_with_length(tx_bytes.len() as u32);
    tx_array.copy_from(&tx_bytes);

    // Send via Phantom
    let sign_fn = Reflect::get(&solana, &JsValue::from_str("signAndSendTransaction"))
        .map_err(|_| "No signAndSendTransaction method")?;

    let sign_fn: js_sys::Function = sign_fn.dyn_into()
        .map_err(|_| "signAndSendTransaction is not a function")?;

    let promise = sign_fn.call1(&solana, &tx_array.into())
        .map_err(|e| format!("Sign call failed: {:?}", e))?;

    let promise: Promise = promise.dyn_into()
        .map_err(|_| "Not a promise")?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Transaction rejected: {:?}", e))?;

    let signature = Reflect::get(&result, &JsValue::from_str("signature"))
        .ok()
        .and_then(|s| s.as_string())
        .ok_or("No signature in response")?;

    Ok(signature)
}

/// Build transaction bytes with reset + checkpoint + deploy instructions
/// v0.5: Checkpoint is required between reset and deploy to claim previous round rewards
fn build_play_transaction_bytes(
    authority: &str,
    fee_collector: &str,
    round_id: u64,
    amount_lamports: u64,
    selected_squares: &[u8],
    blockhash: &str,
) -> Result<Vec<u8>, String> {
    // Calculate all PDAs
    let board = board_pda();
    let config = config_pda();
    let mint = mint_pda();
    let treasury = treasury_pda();
    let treasury_tokens = treasury_tokens_pda();
    let round = round_pda(round_id);
    let round_next = round_pda(round_id + 1);
    let miner = miner_pda(authority);
    let automation = automation_pda(authority);

    // Build squares bitmask for deploy
    let squares_mask: u32 = selected_squares.iter().fold(0u32, |acc, &sq| acc | (1 << sq));

    // Instruction data
    let reset_data = vec![RESET_DISCRIMINATOR];
    let checkpoint_data = vec![CHECKPOINT_DISCRIMINATOR];
    let mut deploy_data = vec![DEPLOY_DISCRIMINATOR];
    deploy_data.extend_from_slice(&amount_lamports.to_le_bytes());
    deploy_data.extend_from_slice(&squares_mask.to_le_bytes());

    // Build unique accounts list
    // Order matters for Solana transaction format
    let mut unique_accounts: Vec<String> = vec![authority.to_string()]; // Fee payer first

    // Reset accounts (from sdk.rs):
    // signer, board, config, fee_collector, mint, round, round_next, top_miner, treasury, treasury_tokens, system, token_program, ore_program, slot_hashes
    let reset_accounts = vec![
        authority.to_string(),      // signer
        board.clone(),              // board
        config.clone(),             // config
        fee_collector.to_string(),  // fee_collector
        mint.clone(),               // mint
        round.clone(),              // round (current)
        round_next.clone(),         // round_next
        authority.to_string(),      // top_miner (placeholder)
        treasury.clone(),           // treasury
        treasury_tokens.clone(),    // treasury_tokens
        SYSTEM_PROGRAM.to_string(), // system_program
        TOKEN_PROGRAM.to_string(),  // token_program
        PROGRAM_ID.to_string(),     // ore_program (skill)
        SLOT_HASHES_SYSVAR.to_string(), // slot_hashes
    ];

    // Checkpoint accounts (from sdk.rs):
    // signer, board, miner, round, treasury, system
    let checkpoint_accounts = vec![
        authority.to_string(),      // signer
        board.clone(),              // board
        miner.clone(),              // miner
        round.clone(),              // round (current, to checkpoint)
        treasury.clone(),           // treasury
        SYSTEM_PROGRAM.to_string(), // system_program
    ];

    // Deploy accounts (from sdk.rs):
    // signer, authority, automation, board, miner, round, system
    let deploy_accounts = vec![
        authority.to_string(),      // signer
        authority.to_string(),      // authority
        automation.clone(),         // automation
        board.clone(),              // board
        miner.clone(),              // miner
        round_next.clone(),         // round (next round after reset)
        SYSTEM_PROGRAM.to_string(), // system_program
    ];

    // Build unique accounts, tracking indices
    for acc in reset_accounts.iter().chain(checkpoint_accounts.iter()).chain(deploy_accounts.iter()) {
        if !unique_accounts.contains(acc) {
            unique_accounts.push(acc.clone());
        }
    }

    // Get indices for each instruction's accounts
    let reset_indices: Vec<u8> = reset_accounts.iter()
        .map(|a| unique_accounts.iter().position(|x| x == a).unwrap() as u8)
        .collect();

    let checkpoint_indices: Vec<u8> = checkpoint_accounts.iter()
        .map(|a| unique_accounts.iter().position(|x| x == a).unwrap() as u8)
        .collect();

    let deploy_indices: Vec<u8> = deploy_accounts.iter()
        .map(|a| unique_accounts.iter().position(|x| x == a).unwrap() as u8)
        .collect();

    let program_idx = unique_accounts.iter().position(|a| a == PROGRAM_ID).unwrap() as u8;

    // Build message
    let mut message = Vec::new();

    // Header: [num_signers, num_readonly_signed, num_readonly_unsigned]
    message.push(1u8); // 1 signer (authority)
    message.push(0u8); // 0 readonly signed
    // Count readonly unsigned: token_program, slot_hashes, ore_program (if not writable elsewhere)
    message.push(3u8); // readonly unsigned accounts

    // Account addresses
    message.extend(compact_u16(unique_accounts.len() as u16));
    for account in &unique_accounts {
        let bytes = bs58::decode(account).into_vec().map_err(|e| e.to_string())?;
        if bytes.len() != 32 {
            return Err(format!("Invalid pubkey: {}", account));
        }
        message.extend(&bytes);
    }

    // Blockhash
    let blockhash_bytes = bs58::decode(blockhash).into_vec().map_err(|e| e.to_string())?;
    message.extend(&blockhash_bytes);

    // Instructions (3 instructions: reset + checkpoint + deploy)
    message.extend(compact_u16(3u16));

    // 1. Reset instruction
    message.push(program_idx);
    message.extend(compact_u16(reset_indices.len() as u16));
    message.extend(&reset_indices);
    message.extend(compact_u16(reset_data.len() as u16));
    message.extend(&reset_data);

    // 2. Checkpoint instruction (claim rewards from previous round)
    message.push(program_idx);
    message.extend(compact_u16(checkpoint_indices.len() as u16));
    message.extend(&checkpoint_indices);
    message.extend(compact_u16(checkpoint_data.len() as u16));
    message.extend(&checkpoint_data);

    // 3. Deploy instruction
    message.push(program_idx);
    message.extend(compact_u16(deploy_indices.len() as u16));
    message.extend(&deploy_indices);
    message.extend(compact_u16(deploy_data.len() as u16));
    message.extend(&deploy_data);

    // Prepend signature count (0 - wallet will add)
    let mut tx = vec![0u8];
    tx.extend(&message);

    Ok(tx)
}

#[cfg(not(feature = "web"))]
pub async fn play_transaction(
    _authority: &str,
    _amount_lamports: u64,
    _selected_squares: &[u8],
) -> Result<String, String> {
    Err("Play only available in web mode".to_string())
}

// ============ CLAIM TRANSACTIONS ============

const CLAIM_SOL_DISCRIMINATOR: u8 = 3;
const CLAIM_ORE_DISCRIMINATOR: u8 = 4;

/// Claim SOL rewards
#[cfg(feature = "web")]
pub async fn claim_sol_transaction(authority: &str) -> Result<String, String> {
    let miner = miner_pda(authority);
    let blockhash = fetch_recent_blockhash(RPC_URL).await?;

    let accounts = vec![
        (authority, true, true),      // signer, writable
        (&miner as &str, true, false), // miner, writable
        (SYSTEM_PROGRAM, false, false), // system_program, readonly
    ];

    let ix_data = vec![CLAIM_SOL_DISCRIMINATOR];

    let tx_bytes = build_transaction_bytes(
        authority,
        &accounts,
        PROGRAM_ID,
        &ix_data,
        &blockhash,
    )?;

    send_transaction_phantom(&tx_bytes).await
}

/// Claim ORE (SKILL) token rewards
#[cfg(feature = "web")]
pub async fn claim_ore_transaction(authority: &str) -> Result<String, String> {
    let miner = miner_pda(authority);
    let treasury = treasury_pda();
    let mint = mint_pda();
    let treasury_tokens = treasury_tokens_pda();

    // Derive recipient's associated token account
    let recipient_ata = derive_associated_token_account(authority, &mint);

    let blockhash = fetch_recent_blockhash(RPC_URL).await?;

    // Accounts from sdk.rs claim_ore:
    // signer, miner, mint, recipient, treasury, treasury_tokens, system, token_program, ata_program
    let accounts = vec![
        (authority, true, true),                   // signer
        (&miner as &str, true, false),             // miner
        (&mint as &str, false, false),             // mint (readonly)
        (&recipient_ata as &str, true, false),     // recipient ATA
        (&treasury as &str, true, false),          // treasury
        (&treasury_tokens as &str, true, false),   // treasury_tokens
        (SYSTEM_PROGRAM, false, false),            // system_program
        (TOKEN_PROGRAM, false, false),             // token_program
        (ASSOCIATED_TOKEN_PROGRAM, false, false),  // ata_program
    ];

    let ix_data = vec![CLAIM_ORE_DISCRIMINATOR];

    let tx_bytes = build_transaction_bytes(
        authority,
        &accounts,
        PROGRAM_ID,
        &ix_data,
        &blockhash,
    )?;

    send_transaction_phantom(&tx_bytes).await
}

const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// Derive associated token account address
fn derive_associated_token_account(owner: &str, mint: &str) -> String {
    let owner_bytes = bs58::decode(owner).into_vec().unwrap_or_default();
    let mint_bytes = bs58::decode(mint).into_vec().unwrap_or_default();
    let ata_program_bytes = bs58::decode(ASSOCIATED_TOKEN_PROGRAM).into_vec().unwrap_or_default();
    let token_program_bytes = bs58::decode(TOKEN_PROGRAM).into_vec().unwrap_or_default();

    derive_pda(
        &[&owner_bytes, &token_program_bytes, &mint_bytes],
        ASSOCIATED_TOKEN_PROGRAM,
    )
}

/// Generic send transaction via Phantom
#[cfg(feature = "web")]
async fn send_transaction_phantom(tx_bytes: &[u8]) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;

    let solana = Reflect::get(&window, &JsValue::from_str("solana"))
        .map_err(|_| "Phantom not found")?;

    if solana.is_undefined() {
        return Err("Phantom wallet not connected".to_string());
    }

    let tx_array = Uint8Array::new_with_length(tx_bytes.len() as u32);
    tx_array.copy_from(tx_bytes);

    let sign_fn = Reflect::get(&solana, &JsValue::from_str("signAndSendTransaction"))
        .map_err(|_| "No signAndSendTransaction method")?;

    let sign_fn: js_sys::Function = sign_fn.dyn_into()
        .map_err(|_| "signAndSendTransaction is not a function")?;

    let promise = sign_fn.call1(&solana, &tx_array.into())
        .map_err(|e| format!("Sign call failed: {:?}", e))?;

    let promise: Promise = promise.dyn_into()
        .map_err(|_| "Not a promise")?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Transaction rejected: {:?}", e))?;

    let signature = Reflect::get(&result, &JsValue::from_str("signature"))
        .ok()
        .and_then(|s| s.as_string())
        .ok_or("No signature in response")?;

    Ok(signature)
}

#[cfg(not(feature = "web"))]
pub async fn claim_sol_transaction(_authority: &str) -> Result<String, String> {
    Err("Claim only available in web mode".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn claim_ore_transaction(_authority: &str) -> Result<String, String> {
    Err("Claim only available in web mode".to_string())
}
