use std::{collections::HashMap, str::FromStr};

// Entropy API only needed for legacy admin commands (new_var)
use entropy_api::state as entropy_state;
use jup_swap::{
    quote::QuoteRequest,
    swap::SwapRequest,
    transaction_config::{DynamicSlippageSettings, TransactionConfig},
    JupiterSwapApiClient,
};
use skill_api::prelude::*;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    client_error::{reqwest::StatusCode, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    compute_budget::ComputeBudgetInstruction,
    message::{v0::Message, VersionedMessage},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    rent::Rent,
    signature::{read_keypair_file, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use solana_sdk::{keccak, pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token::amount_to_ui_amount;
use steel::{AccountDeserialize, AccountMeta, Clock, Discriminator, Instruction};

#[tokio::main]
async fn main() {
    // Read keypair from file
    let payer =
        read_keypair_file(&std::env::var("KEYPAIR").expect("Missing KEYPAIR env var")).unwrap();

    // Build transaction
    let rpc = RpcClient::new(std::env::var("RPC").expect("Missing RPC env var"));
    match std::env::var("COMMAND")
        .expect("Missing COMMAND env var")
        .as_str()
    {
        "automations" => {
            log_automations(&rpc).await.unwrap();
        }
        "clock" => {
            log_clock(&rpc).await.unwrap();
        }
        "claim" => {
            claim(&rpc, &payer).await.unwrap();
        }
        "board" => {
            log_board(&rpc).await.unwrap();
        }
        "config" => {
            log_config(&rpc).await.unwrap();
        }
        "buyback" => {
            buyback(&rpc, &payer).await.unwrap();
        }
        "reset" => {
            reset(&rpc, &payer).await.unwrap();
        }
        "treasury" => {
            log_treasury(&rpc).await.unwrap();
        }
        "miner" => {
            log_miner(&rpc, &payer).await.unwrap();
        }
        // "pool" => {
        //     log_meteora_pool(&rpc).await.unwrap();
        // }
        "deploy" => {
            deploy(&rpc, &payer).await.unwrap();
        }
        "play" => {
            play(&rpc, &payer).await.unwrap();
        }
        "stake" => {
            log_stake(&rpc, &payer).await.unwrap();
        }
        "deploy_all" => {
            deploy_all(&rpc, &payer).await.unwrap();
        }
        "round" => {
            log_round(&rpc).await.unwrap();
        }
        "set_admin" => {
            set_admin(&rpc, &payer).await.unwrap();
        }
        "set_fee_collector" => {
            set_fee_collector(&rpc, &payer).await.unwrap();
        }
        "ata" => {
            ata(&rpc, &payer).await.unwrap();
        }
        "checkpoint" => {
            checkpoint(&rpc, &payer).await.unwrap();
        }
        "checkpoint_all" => {
            checkpoint_all(&rpc, &payer).await.unwrap();
        }
        "close_all" => {
            close_all(&rpc, &payer).await.unwrap();
        }
        "participating_miners" => {
            participating_miners(&rpc).await.unwrap();
        }
        "new_var" => {
            new_var(&rpc, &payer).await.unwrap();
        }
        "set_admin_fee" => {
            set_admin_fee(&rpc, &payer).await.unwrap();
        }
        "set_swap_program" => {
            set_swap_program(&rpc, &payer).await.unwrap();
        }
        "set_var_address" => {
            set_var_address(&rpc, &payer).await.unwrap();
        }
        "keys" => {
            keys().await.unwrap();
        }
        "lut" => {
            lut(&rpc, &payer).await.unwrap();
        }
        "liq" => {
            liq(&rpc, &payer).await.unwrap();
        }
        "migrate_automation" => {
            migrate_automation(&rpc, &payer).await.unwrap();
        }
        "automation" => {
            log_automation(&rpc).await.unwrap();
        }
        "init" => {
            init(&rpc, &payer).await.unwrap();
        }
        // v0.2 Skill System
        "predict" => {
            predict(&rpc, &payer).await.unwrap();
        }
        "skill" => {
            log_skill(&rpc, &payer).await.unwrap();
        }
        _ => panic!("Invalid command"),
    };
}

async fn init(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    // Read optional parameters from environment variables.
    let admin = std::env::var("ADMIN")
        .map(|s| Pubkey::from_str(&s).expect("Invalid ADMIN"))
        .unwrap_or(payer.pubkey());

    let fee_collector = std::env::var("FEE_COLLECTOR")
        .map(|s| Pubkey::from_str(&s).expect("Invalid FEE_COLLECTOR"))
        .unwrap_or(payer.pubkey());

    let var_address = std::env::var("VAR_ADDRESS")
        .map(|s| Pubkey::from_str(&s).expect("Invalid VAR_ADDRESS"))
        .unwrap_or(Pubkey::default());

    // Build and submit initialize instruction.
    let ix = skill_api::sdk::initialize(payer.pubkey(), admin, fee_collector, var_address);
    let sig = submit_transaction(rpc, payer, &[ix]).await?;

    // Output created addresses.
    let board_address = skill_api::state::board_pda().0;
    let config_address = skill_api::state::config_pda().0;
    let treasury_address = skill_api::state::treasury_pda().0;
    let mint_address = skill_api::state::mint_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &mint_address);

    println!();
    println!("Initialization complete!");
    println!("Transaction: {}", sig);
    println!();
    println!("Created accounts:");
    println!("  Board:            {}", board_address);
    println!("  Config:           {}", config_address);
    println!("  Treasury:         {}", treasury_address);
    println!("  SKILL Mint:       {}", mint_address);
    println!("  Treasury Tokens:  {}", treasury_tokens_address);
    println!();
    println!("Configuration:");
    println!("  Admin:            {}", admin);
    println!("  Fee Collector:    {}", fee_collector);
    println!("  Var Address:      {}", var_address);

    Ok(())
}

async fn liq(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let manager = pubkey!("DJqfQWB8tZE6fzqWa8okncDh7ciTuD8QQKp1ssNETWee");
    let wrap_ix = skill_api::sdk::wrap(payer.pubkey());
    let liq_ix = skill_api::sdk::liq(payer.pubkey(), manager);
    submit_transaction(rpc, payer, &[wrap_ix, liq_ix]).await?;
    Ok(())
}

async fn migrate_automation(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authorities = [
        pubkey!("HSB6HB184xHLsEBia2VR3rdqrme9MWZR9tVPLT3Ndda2"),
        pubkey!("3SrTpJEsTonUf9Ew7eGSi1xhNN6gqaKbZUc9ncFcGz7b"),
        pubkey!("Bwyuj9ybgSTtPkhvCFxL1A7uV9SiA75nb55qBF6pFMKz"),
    ];
    for authority in authorities {
        let ix = skill_api::sdk::migrate_automation(payer.pubkey(), authority);
        if let Err(e) = submit_transaction_no_confirm(rpc, payer, &[ix]).await {
            println!("Error submitting transaction: {:?}", e);
        }
    }
    Ok(())
}

async fn lut(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let recent_slot = rpc.get_slot().await? - 4;
    let (ix, lut_address) = solana_address_lookup_table_interface::instruction::create_lookup_table(
        payer.pubkey(),
        payer.pubkey(),
        recent_slot,
    );
    let ex_ix = solana_address_lookup_table_interface::instruction::extend_lookup_table(
        lut_address,
        payer.pubkey(),
        Some(payer.pubkey()),
        vec![
            pubkey!("HNWhK5f8RMWBqcA7mXJPaxdTPGrha3rrqUrri7HSKb3T"),
            pubkey!("2wQ7J46uwK3VyrmAYe5E8KhCjTg8CTaFimh1ty2huuyY"),
            pubkey!("DJqfQWB8tZE6fzqWa8okncDh7ciTuD8QQKp1ssNETWee"),
            pubkey!("HLaJ3RiyoaxQzwJQbU2Gc5RTZtx8HKAMJgkf57qdgpFJ"),
            pubkey!("8yS5zJTZa1Q1zQ1jsEAUnjAyMZfsNwvrgbDQp1ky2dr"),
            pubkey!("7qBS6huLjjGyrnMMBNXpLZA73yiGc6ao9znj7f9RpF1L"),
            pubkey!("3Mt1bpU3fnSXyPEm66HKKXyQTpLWrwYziPLqwTqK4ZT7"),
            pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo"),
            pubkey!("oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp"),
            pubkey!("So11111111111111111111111111111111111111112"),
            pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
            pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
            pubkey!("D1ZN9Wj1fRSUQfCjhvnu1hqDMT7hzjzBBpi12nVniYD6"),
            pubkey!("8kqLv9cBUDCYEKCL3Dj2MkeXX3tdCqT8KZ3gpYp8BnGP"),
            pubkey!("H38TVzkjAiAhBZR5SksbW8XDXP3N1ez4Tuna7uAW1Tsw"),
            pubkey!("11111111111111111111111111111111"),
            pubkey!("SysvarRent111111111111111111111111111111111"),
        ],
    );
    let ix_1 = Instruction {
        program_id: ix.program_id,
        accounts: ix
            .accounts
            .iter()
            .map(|a| AccountMeta::new(a.pubkey, a.is_signer))
            .collect(),
        data: ix.data,
    };
    let ix_2 = Instruction {
        program_id: ex_ix.program_id,
        accounts: ex_ix
            .accounts
            .iter()
            .map(|a| AccountMeta::new(a.pubkey, a.is_signer))
            .collect(),
        data: ex_ix.data,
    };
    submit_transaction(rpc, payer, &[ix_1, ix_2]).await?;
    println!("LUT address: {}", lut_address);
    Ok(())
}

async fn set_admin_fee(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let admin_fee = std::env::var("ADMIN_FEE").expect("Missing ADMIN_FEE env var");
    let admin_fee = u64::from_str(&admin_fee).expect("Invalid ADMIN_FEE");
    let ix = skill_api::sdk::set_admin_fee(payer.pubkey(), admin_fee);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_var_address(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let new_var_address = std::env::var("VAR").expect("Missing VAR env var");
    let new_var_address = Pubkey::from_str(&new_var_address).expect("Invalid VAR");
    let ix = skill_api::sdk::set_var_address(payer.pubkey(), new_var_address);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn new_var(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let provider = std::env::var("PROVIDER").expect("Missing PROVIDER env var");
    let provider = Pubkey::from_str(&provider).expect("Invalid PROVIDER");
    let commit = std::env::var("COMMIT").expect("Missing COMMIT env var");
    let commit = keccak::Hash::from_str(&commit).expect("Invalid COMMIT");
    let samples = std::env::var("SAMPLES").expect("Missing SAMPLES env var");
    let samples = u64::from_str(&samples).expect("Invalid SAMPLES");
    let board_address = board_pda().0;
    let var_address = entropy_state::var_pda(board_address, 0).0;
    println!("Var address: {}", var_address);
    let ix = skill_api::sdk::new_var(payer.pubkey(), provider, 0, commit.to_bytes(), samples);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn participating_miners(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let round_id = std::env::var("ID").expect("Missing ID env var");
    let round_id = u64::from_str(&round_id).expect("Invalid ID");
    let miners = get_miners_participating(rpc, round_id).await?;
    for (i, (_address, miner)) in miners.iter().enumerate() {
        println!("{}: {}", i, miner.authority);
    }
    Ok(())
}

async fn log_stake(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let staker_address = skill_api::state::stake_pda(authority).0;
    let stake = get_stake(rpc, authority).await?;
    println!("Stake");
    println!("  address: {}", staker_address);
    println!("  authority: {}", authority);
    println!(
        "  balance: {} ORE",
        amount_to_ui_amount(stake.balance, TOKEN_DECIMALS)
    );
    println!("  last_claim_at: {}", stake.last_claim_at);
    println!("  last_deposit_at: {}", stake.last_deposit_at);
    println!("  last_withdraw_at: {}", stake.last_withdraw_at);
    println!(
        "  rewards_factor: {}",
        stake.rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  rewards: {} ORE",
        amount_to_ui_amount(stake.rewards, TOKEN_DECIMALS)
    );
    println!(
        "  lifetime_rewards: {} ORE",
        amount_to_ui_amount(stake.lifetime_rewards, TOKEN_DECIMALS)
    );

    Ok(())
}

async fn ata(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let user = pubkey!("FgZFnb3bi7QexKCdXWPwWy91eocUD7JCFySHb83vLoPD");
    let token = pubkey!("8H8rPiWW4iTFCfEkSnf7jpqeNpFfvdH9gLouAL3Fe2Zx");
    let ata = get_associated_token_address(&user, &token);
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &user,
        &token,
        &spl_token::ID,
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    let account = rpc.get_account(&ata).await?;
    println!("ATA: {}", ata);
    println!("Account: {:?}", account);
    Ok(())
}

async fn keys() -> Result<(), anyhow::Error> {
    let treasury_address = skill_api::state::treasury_pda().0;
    let config_address = skill_api::state::config_pda().0;
    let board_address = skill_api::state::board_pda().0;
    let address = pubkey!("pqspJ298ryBjazPAr95J9sULCVpZe3HbZTWkbC1zrkS");
    let miner_address = skill_api::state::miner_pda(address).0;
    let round = round_pda(31460).0;
    println!("Round: {}", round);
    println!("Treasury: {}", treasury_address);
    println!("Config: {}", config_address);
    println!("Board: {}", board_address);
    println!("Miner: {}", miner_address);
    Ok(())
}

async fn claim(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let ix_sol = skill_api::sdk::claim_sol(payer.pubkey());
    let ix_ore = skill_api::sdk::claim_ore(payer.pubkey());
    submit_transaction(rpc, payer, &[ix_sol, ix_ore]).await?;
    Ok(())
}

async fn buyback(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    // Get swap amount.
    let treasury = get_treasury(rpc).await?;
    let amount = treasury.balance.min(10 * LAMPORTS_PER_SOL);

    // Build quote request.
    const INPUT_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
    const OUTPUT_MINT: Pubkey = pubkey!("oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp");
    let api_base_url =
        std::env::var("API_BASE_URL").unwrap_or("https://lite-api.jup.ag/swap/v1".into());
    let jupiter_swap_api_client = JupiterSwapApiClient::new(api_base_url);
    let quote_request = QuoteRequest {
        amount,
        input_mint: INPUT_MINT,
        output_mint: OUTPUT_MINT,
        max_accounts: Some(55),
        ..QuoteRequest::default()
    };

    // GET /quote
    let quote_response = match jupiter_swap_api_client.quote(&quote_request).await {
        Ok(quote_response) => quote_response,
        Err(e) => {
            println!("quote failed: {e:#?}");
            return Err(anyhow::anyhow!("quote failed: {e:#?}"));
        }
    };

    // GET /swap/instructions
    let treasury_address = skill_api::state::treasury_pda().0;
    let response = jupiter_swap_api_client
        .swap_instructions(&SwapRequest {
            user_public_key: treasury_address,
            quote_response,
            config: TransactionConfig {
                skip_user_accounts_rpc_calls: true,
                wrap_and_unwrap_sol: false,
                dynamic_compute_unit_limit: true,
                dynamic_slippage: Some(DynamicSlippageSettings {
                    min_bps: Some(50),
                    max_bps: Some(1000),
                }),
                ..TransactionConfig::default()
            },
        })
        .await
        .unwrap();

    let address_lookup_table_accounts =
        get_address_lookup_table_accounts(rpc, response.address_lookup_table_addresses)
            .await
            .unwrap();

    // Build transaction.
    let wrap_ix = skill_api::sdk::wrap(payer.pubkey());
    let buyback_ix = skill_api::sdk::buyback(
        payer.pubkey(),
        &response.swap_instruction.accounts,
        &response.swap_instruction.data,
    );
    simulate_transaction_with_address_lookup_tables(
        rpc,
        payer,
        &[wrap_ix, buyback_ix],
        address_lookup_table_accounts,
    )
    .await;

    Ok(())
}

#[allow(dead_code)]
pub async fn get_address_lookup_table_accounts(
    rpc_client: &RpcClient,
    addresses: Vec<Pubkey>,
) -> Result<Vec<AddressLookupTableAccount>, anyhow::Error> {
    let mut accounts = Vec::new();
    for key in addresses {
        if let Ok(account) = rpc_client.get_account(&key).await {
            if let Ok(address_lookup_table_account) = AddressLookupTable::deserialize(&account.data)
            {
                accounts.push(AddressLookupTableAccount {
                    key,
                    addresses: address_lookup_table_account.addresses.to_vec(),
                });
            }
        }
    }
    Ok(accounts)
}

/// Schelling Point: Reset determines winner by majority vote (no entropy needed)
async fn reset(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let board = get_board(rpc).await?;
    let config = get_config(rpc).await?;

    // Get current round to show what will be the winning square
    if let Ok(round) = get_round(rpc, board.round_id).await {
        // Find the winning square (argmax of deployed)
        let (winning_square, max_deployed) = round
            .deployed
            .iter()
            .enumerate()
            .max_by(|(i1, v1), (i2, v2)| v1.cmp(v2).then_with(|| i2.cmp(i1)))
            .map(|(i, &v)| (i, v))
            .unwrap_or((0, 0));

        println!("Schelling Point Reset");
        println!("  Round ID: {}", board.round_id);
        println!("  Total deployed: {} lamports", round.total_deployed);
        println!("  Winning square: #{} ({} lamports)", winning_square, max_deployed);
        println!("  Miners on winner: {}", round.count[winning_square]);
    }

    let reset_ix = skill_api::sdk::reset(
        payer.pubkey(),
        config.fee_collector,
        board.round_id,
        Pubkey::default(),
    );
    let sig = submit_transaction(rpc, payer, &[reset_ix]).await?;
    println!("Reset transaction: {}", sig);

    Ok(())
}

async fn deploy(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let amount = std::env::var("AMOUNT").expect("Missing AMOUNT env var");
    let amount = u64::from_str(&amount).expect("Invalid AMOUNT");
    let square_id = std::env::var("SQUARE").expect("Missing SQUARE env var");
    let square_id = u64::from_str(&square_id).expect("Invalid SQUARE");
    let board = get_board(rpc).await?;
    let mut squares = [false; 25];
    squares[square_id as usize] = true;
    let ix = skill_api::sdk::deploy(
        payer.pubkey(),
        payer.pubkey(),
        amount,
        board.round_id,
        squares,
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

/// Smart play command that automatically handles round transitions.
/// If the current round has ended, it will reset first, then deploy.
/// This is the main entry point for players - no external crank needed!
async fn play(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    // Parse arguments
    let amount = std::env::var("AMOUNT").expect("Missing AMOUNT env var");
    let amount = u64::from_str(&amount).expect("Invalid AMOUNT");
    let square_id = std::env::var("SQUARE").expect("Missing SQUARE env var");
    let square_id = u64::from_str(&square_id).expect("Invalid SQUARE");

    // Get current state
    let board = get_board(rpc).await?;
    let config = get_config(rpc).await?;
    let clock = get_clock(rpc).await?;

    // Check if round has ended
    let round_ended = board.end_slot != u64::MAX
        && clock.slot >= board.end_slot + INTERMISSION_SLOTS;

    let mut squares = [false; 25];
    squares[square_id as usize] = true;

    if round_ended {
        println!("═══════════════════════════════════════════════════════════");
        println!("  Round {} has ended. Resetting and deploying...", board.round_id);
        println!("═══════════════════════════════════════════════════════════");

        // Show what will happen
        if let Ok(round) = get_round(rpc, board.round_id).await {
            let (winning_square, max_deployed) = round
                .deployed
                .iter()
                .enumerate()
                .max_by(|(i1, v1), (i2, v2)| v1.cmp(v2).then_with(|| i2.cmp(i1)))
                .map(|(i, &v)| (i, v))
                .unwrap_or((0, 0));

            println!("  Winning square: #{} ({} lamports)", winning_square, max_deployed);
            println!("  Total deployed: {} lamports", round.total_deployed);
            println!("  Winners count: {}", round.count[winning_square]);
        }

        println!("  Deploying {} lamports to square #{} in round {}", amount, square_id, board.round_id + 1);
        println!("═══════════════════════════════════════════════════════════");
    } else if board.end_slot == u64::MAX {
        println!("═══════════════════════════════════════════════════════════");
        println!("  No active round. Your deploy will start round {}!", board.round_id);
        println!("  Deploying {} lamports to square #{}", amount, square_id);
        println!("═══════════════════════════════════════════════════════════");
    } else {
        let slots_remaining = board.end_slot.saturating_sub(clock.slot);
        let seconds_remaining = slots_remaining * 400 / 1000; // ~400ms per slot
        println!("═══════════════════════════════════════════════════════════");
        println!("  Round {} active - {} slots (~{}s) remaining", board.round_id, slots_remaining, seconds_remaining);
        println!("  Deploying {} lamports to square #{}", amount, square_id);
        println!("═══════════════════════════════════════════════════════════");
    }

    // Build and submit transaction (reset + deploy if needed)
    let instructions = skill_api::sdk::play(
        payer.pubkey(),
        amount,
        squares,
        config.fee_collector,
        board.round_id,
        round_ended,
    );

    let sig = submit_transaction(rpc, payer, &instructions).await?;
    println!("Transaction: {}", sig);

    // Show updated state
    let new_board = get_board(rpc).await?;
    println!("\nBoard state after play:");
    println!("  Round ID: {}", new_board.round_id);
    if new_board.end_slot != u64::MAX {
        let new_clock = get_clock(rpc).await?;
        let slots_remaining = new_board.end_slot.saturating_sub(new_clock.slot);
        println!("  Slots remaining: {}", slots_remaining);
    }

    Ok(())
}

async fn deploy_all(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let amount = std::env::var("AMOUNT").expect("Missing AMOUNT env var");
    let amount = u64::from_str(&amount).expect("Invalid AMOUNT");
    let board = get_board(rpc).await?;
    let squares = [true; 25];
    let ix = skill_api::sdk::deploy(
        payer.pubkey(),
        payer.pubkey(),
        board.round_id,
        amount,
        squares,
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_admin(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let ix = skill_api::sdk::set_admin(payer.pubkey(), payer.pubkey());
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_swap_program(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let swap_program = std::env::var("SWAP_PROGRAM").expect("Missing SWAP_PROGRAM env var");
    let swap_program = Pubkey::from_str(&swap_program).expect("Invalid SWAP_PROGRAM");
    let ix = skill_api::sdk::set_swap_program(payer.pubkey(), swap_program);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_fee_collector(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let fee_collector = std::env::var("FEE_COLLECTOR").expect("Missing FEE_COLLECTOR env var");
    let fee_collector = Pubkey::from_str(&fee_collector).expect("Invalid FEE_COLLECTOR");
    let ix = skill_api::sdk::set_fee_collector(payer.pubkey(), fee_collector);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn checkpoint(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let miner = get_miner(rpc, authority).await?;
    let ix = skill_api::sdk::checkpoint(payer.pubkey(), authority, miner.round_id);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn checkpoint_all(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let clock = get_clock(rpc).await?;
    let miners = get_miners(rpc).await?;
    let mut expiry_slots = HashMap::new();
    let mut ixs = vec![];
    for (i, (_address, miner)) in miners.iter().enumerate() {
        if miner.checkpoint_id < miner.round_id {
            // Log the expiry slot for the round.
            if !expiry_slots.contains_key(&miner.round_id) {
                if let Ok(round) = get_round(rpc, miner.round_id).await {
                    expiry_slots.insert(miner.round_id, round.expires_at);
                }
            }

            // Get the expiry slot for the round.
            let Some(expires_at) = expiry_slots.get(&miner.round_id) else {
                continue;
            };

            // If we are in fee collection period, checkpoint the miner.
            if clock.slot >= expires_at - TWELVE_HOURS_SLOTS {
                println!(
                    "[{}/{}] Checkpoint miner: {} ({} s)",
                    i + 1,
                    miners.len(),
                    miner.authority,
                    (expires_at - clock.slot) as f64 * 0.4
                );
                ixs.push(skill_api::sdk::checkpoint(
                    payer.pubkey(),
                    miner.authority,
                    miner.round_id,
                ));
            }
        }
    }

    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(10, ixs.len()))
            .collect::<Vec<Instruction>>();
        submit_transaction(rpc, payer, &batch).await?;
    }

    Ok(())
}

async fn close_all(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let rounds = get_rounds(rpc).await?;
    let mut ixs = vec![];
    let clock = get_clock(rpc).await?;
    for (_i, (_address, round)) in rounds.iter().enumerate() {
        if clock.slot >= round.expires_at {
            ixs.push(skill_api::sdk::close(
                payer.pubkey(),
                round.id,
                round.rent_payer,
            ));
        }
    }

    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(12, ixs.len()))
            .collect::<Vec<Instruction>>();
        // simulate_transaction(rpc, payer, &batch).await;
        submit_transaction(rpc, payer, &batch).await?;
    }

    Ok(())
}

// async fn log_meteora_pool(rpc: &RpcClient) -> Result<(), anyhow::Error> {
//     let address = pubkey!("GgaDTFbqdgjoZz3FP7zrtofGwnRS4E6MCzmmD5Ni1Mxj");
//     let pool = get_meteora_pool(rpc, address).await?;
//     let vault_a = get_meteora_vault(rpc, pool.a_vault).await?;
//     let vault_b = get_meteora_vault(rpc, pool.b_vault).await?;

//     println!("Pool");
//     println!("  address: {}", address);
//     println!("  lp_mint: {}", pool.lp_mint);
//     println!("  token_a_mint: {}", pool.token_a_mint);
//     println!("  token_b_mint: {}", pool.token_b_mint);
//     println!("  a_vault: {}", pool.a_vault);
//     println!("  b_vault: {}", pool.b_vault);
//     println!("  a_token_vault: {}", vault_a.token_vault);
//     println!("  b_token_vault: {}", vault_b.token_vault);
//     println!("  a_vault_lp_mint: {}", vault_a.lp_mint);
//     println!("  b_vault_lp_mint: {}", vault_b.lp_mint);
//     println!("  a_vault_lp: {}", pool.a_vault_lp);
//     println!("  b_vault_lp: {}", pool.b_vault_lp);
//     println!("  protocol_token_fee: {}", pool.protocol_token_b_fee);

//     // pool: *pool.key,
//     // user_source_token: *user_source_token.key,
//     // user_destination_token: *user_destination_token.key,
//     // a_vault: *a_vault.key,
//     // b_vault: *b_vault.key,
//     // a_token_vault: *a_token_vault.key,
//     // b_token_vault: *b_token_vault.key,
//     // a_vault_lp_mint: *a_vault_lp_mint.key,
//     // b_vault_lp_mint: *b_vault_lp_mint.key,
//     // a_vault_lp: *a_vault_lp.key,
//     // b_vault_lp: *b_vault_lp.key,
//     // protocol_token_fee: *protocol_token_fee.key,
//     // user: *user.key,
//     // vault_program: *vault_program.key,
//     // token_program: *token_program.key,

//     Ok(())
// }

async fn log_automation(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").expect("Missing AUTHORITY env var");
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let address = automation_pda(authority).0;
    let automation = get_automation(rpc, address).await?;
    let account_balance = rpc.get_balance(&address).await?;
    let size = 8 + std::mem::size_of::<Automation>();
    let required_rent = Rent::default().minimum_balance(size);
    println!("Automation");
    println!("  address: {}", address);
    println!("  amount: {} SOL", automation.amount as f64 / LAMPORTS_PER_SOL as f64);
    println!("  required rent: {} SOL", required_rent as f64 / LAMPORTS_PER_SOL as f64);
    println!("  authority: {}", automation.authority);
    println!("  balance: {} SOL", automation.balance as f64 / LAMPORTS_PER_SOL as f64);
    println!("  lamports: {} SOL", account_balance as f64 / LAMPORTS_PER_SOL as f64);
    println!("  executor: {}", automation.executor);
    println!("  fee: {} SOL", automation.fee as f64 / LAMPORTS_PER_SOL as f64);
    println!("  mask: {}", automation.mask);
    println!("  strategy: {}", automation.strategy);
    println!("  reload: {}", automation.reload);
    Ok(())
}

async fn log_automations(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let automations = get_automations(rpc).await?;
    for (i, (address, automation)) in automations.iter().enumerate() {
        println!("[{}/{}] {}", i + 1, automations.len(), address);
        println!("  authority: {}", automation.authority);
        println!("  balance: {}", automation.balance);
        println!("  executor: {}", automation.executor);
        println!("  fee: {}", automation.fee);
        println!("  mask: {}", automation.mask);
        println!("  strategy: {}", automation.strategy);
        println!();
    }
    Ok(())
}

async fn log_treasury(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let treasury_address = skill_api::state::treasury_pda().0;
    let treasury = get_treasury(rpc).await?;
    println!("Treasury");
    println!("  address: {}", treasury_address);
    println!("  balance: {} SOL", treasury.balance as f64 / LAMPORTS_PER_SOL as f64);
    println!(
        "  motherlode: {} ORE",
        amount_to_ui_amount(treasury.motherlode, TOKEN_DECIMALS)
    );
    println!(
        "  miner_rewards_factor: {}",
        treasury.miner_rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  stake_rewards_factor: {}",
        treasury.stake_rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  total_staked: {} ORE",
        amount_to_ui_amount(treasury.total_staked, TOKEN_DECIMALS)
    );
    println!(
        "  total_unclaimed: {} ORE",
        amount_to_ui_amount(treasury.total_unclaimed, TOKEN_DECIMALS)
    );
    println!(
        "  total_refined: {} ORE",
        amount_to_ui_amount(treasury.total_refined, TOKEN_DECIMALS)
    );
    Ok(())
}

async fn log_round(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let id = std::env::var("ID").expect("Missing ID env var");
    let id = u64::from_str(&id).expect("Invalid ID");
    let round_address = round_pda(id).0;
    let round = get_round(rpc, id).await?;
    println!("Round");
    println!("  Address: {}", round_address);
    println!("  Count: {:?}", round.count);
    println!("  Deployed: {:?}", round.deployed);
    println!("  Expires at: {}", round.expires_at);
    println!("  Id: {:?}", round.id);
    println!("  Motherlode: {}", round.motherlode);
    println!("  Rent payer: {}", round.rent_payer);
    println!("  Slot hash: {:?}", round.slot_hash);
    println!("  Top miner: {:?}", round.top_miner);
    println!("  Top miner reward: {}", round.top_miner_reward);
    println!("  Total deployed: {}", round.total_deployed);
    println!("  Total vaulted: {}", round.total_vaulted);
    println!("  Total winnings: {}", round.total_winnings);
    println!("  Winning square: {}", round.winning_square);
    if round.is_finalized() {
        println!("  Round finalized: yes (slot_hash sampled)");
    } else {
        println!("  Round finalized: no (waiting for reset)");
    }
    Ok(())
}

async fn log_miner(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let miner_address = skill_api::state::miner_pda(authority).0;
    let miner = get_miner(&rpc, authority).await?;
    println!("Miner");
    println!("  address: {}", miner_address);
    println!("  authority: {}", authority);
    println!("  deployed: {:?}", miner.deployed);
    println!("  cumulative: {:?}", miner.cumulative);
    println!("  rewards_sol: {} SOL", miner.rewards_sol as f64 / LAMPORTS_PER_SOL as f64);
    println!(
        "  rewards_ore: {} ORE",
        amount_to_ui_amount(miner.rewards_ore, TOKEN_DECIMALS)
    );
    println!(
        "  refined_ore: {} ORE",
        amount_to_ui_amount(miner.refined_ore, TOKEN_DECIMALS)
    );
    println!("  round_id: {}", miner.round_id);
    println!("  checkpoint_id: {}", miner.checkpoint_id);
    println!(
        "  lifetime_rewards_sol: {} SOL",
        miner.lifetime_rewards_sol as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "  lifetime_rewards_ore: {} ORE",
        amount_to_ui_amount(miner.lifetime_rewards_ore, TOKEN_DECIMALS)
    );
    Ok(())
}

async fn log_clock(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let clock = get_clock(&rpc).await?;
    println!("Clock");
    println!("  slot: {}", clock.slot);
    println!("  epoch_start_timestamp: {}", clock.epoch_start_timestamp);
    println!("  epoch: {}", clock.epoch);
    println!("  leader_schedule_epoch: {}", clock.leader_schedule_epoch);
    println!("  unix_timestamp: {}", clock.unix_timestamp);
    Ok(())
}

async fn log_config(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let config = get_config(&rpc).await?;
    println!("Config");
    println!("  admin: {}", config.admin);
    println!("  bury_authority: {}", config.bury_authority);
    println!("  fee_collector: {}", config.fee_collector);
    println!("  swap_program: {}", config.swap_program);
    println!("  var_address: {}", config.var_address);
    println!("  admin_fee: {}", config.admin_fee);
    Ok(())
}

async fn log_board(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let board = get_board(&rpc).await?;
    let clock = get_clock(&rpc).await?;
    print_board(board, &clock);
    Ok(())
}

fn print_board(board: Board, clock: &Clock) {
    let current_slot = clock.slot;
    println!("Board");
    println!("  Id: {:?}", board.round_id);
    println!("  Start slot: {}", board.start_slot);
    println!("  End slot: {}", board.end_slot);
    println!(
        "  Time remaining: {} sec",
        (board.end_slot.saturating_sub(current_slot) as f64) * 0.4
    );
}

async fn get_automation(rpc: &RpcClient, address: Pubkey) -> Result<Automation, anyhow::Error> {
    let account = rpc.get_account(&address).await?;
    let automation = Automation::try_from_bytes(&account.data)?;
    Ok(*automation)
}

async fn get_automations(rpc: &RpcClient) -> Result<Vec<(Pubkey, Automation)>, anyhow::Error> {
    const REGOLITH_EXECUTOR: Pubkey = pubkey!("HNWhK5f8RMWBqcA7mXJPaxdTPGrha3rrqUrri7HSKb3T");
    let filter = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        56,
        &REGOLITH_EXECUTOR.to_bytes(),
    ));
    let automations = get_program_accounts::<Automation>(rpc, skill_api::ID, vec![filter]).await?;
    Ok(automations)
}

// async fn get_meteora_pool(rpc: &RpcClient, address: Pubkey) -> Result<Pool, anyhow::Error> {
//     let data = rpc.get_account_data(&address).await?;
//     let pool = Pool::from_bytes(&data)?;
//     Ok(pool)
// }

// async fn get_meteora_vault(rpc: &RpcClient, address: Pubkey) -> Result<Vault, anyhow::Error> {
//     let data = rpc.get_account_data(&address).await?;
//     let vault = Vault::from_bytes(&data)?;
//     Ok(vault)
// }

async fn get_board(rpc: &RpcClient) -> Result<Board, anyhow::Error> {
    let board_pda = skill_api::state::board_pda();
    let account = rpc.get_account(&board_pda.0).await?;
    let board = Board::try_from_bytes(&account.data)?;
    Ok(*board)
}

// get_var removed - Schelling Point doesn't need entropy

async fn get_round(rpc: &RpcClient, id: u64) -> Result<Round, anyhow::Error> {
    let round_pda = skill_api::state::round_pda(id);
    let account = rpc.get_account(&round_pda.0).await?;
    let round = Round::try_from_bytes(&account.data)?;
    Ok(*round)
}

async fn get_treasury(rpc: &RpcClient) -> Result<Treasury, anyhow::Error> {
    let treasury_pda = skill_api::state::treasury_pda();
    let account = rpc.get_account(&treasury_pda.0).await?;
    let treasury = Treasury::try_from_bytes(&account.data)?;
    Ok(*treasury)
}

async fn get_config(rpc: &RpcClient) -> Result<Config, anyhow::Error> {
    let config_pda = skill_api::state::config_pda();
    let account = rpc.get_account(&config_pda.0).await?;
    let config = Config::try_from_bytes(&account.data)?;
    Ok(*config)
}

async fn get_miner(rpc: &RpcClient, authority: Pubkey) -> Result<Miner, anyhow::Error> {
    let miner_pda = skill_api::state::miner_pda(authority);
    let account = rpc.get_account(&miner_pda.0).await?;
    let miner = Miner::try_from_bytes(&account.data)?;
    Ok(*miner)
}

async fn get_clock(rpc: &RpcClient) -> Result<Clock, anyhow::Error> {
    let data = rpc.get_account_data(&solana_sdk::sysvar::clock::ID).await?;
    let clock = bincode::deserialize::<Clock>(&data)?;
    Ok(clock)
}

async fn get_stake(rpc: &RpcClient, authority: Pubkey) -> Result<Stake, anyhow::Error> {
    let stake_pda = skill_api::state::stake_pda(authority);
    let account = rpc.get_account(&stake_pda.0).await?;
    let stake = Stake::try_from_bytes(&account.data)?;
    Ok(*stake)
}

async fn get_rounds(rpc: &RpcClient) -> Result<Vec<(Pubkey, Round)>, anyhow::Error> {
    let rounds = get_program_accounts::<Round>(rpc, skill_api::ID, vec![]).await?;
    Ok(rounds)
}

#[allow(dead_code)]
async fn get_miners(rpc: &RpcClient) -> Result<Vec<(Pubkey, Miner)>, anyhow::Error> {
    let miners = get_program_accounts::<Miner>(rpc, skill_api::ID, vec![]).await?;
    Ok(miners)
}

async fn get_miners_participating(
    rpc: &RpcClient,
    round_id: u64,
) -> Result<Vec<(Pubkey, Miner)>, anyhow::Error> {
    let filter = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(512, &round_id.to_le_bytes()));
    let miners = get_program_accounts::<Miner>(rpc, skill_api::ID, vec![filter]).await?;
    Ok(miners)
}

// fn get_winning_square(slot_hash: &[u8]) -> u64 {
//     // Use slot hash to generate a random u64
//     let r1 = u64::from_le_bytes(slot_hash[0..8].try_into().unwrap());
//     let r2 = u64::from_le_bytes(slot_hash[8..16].try_into().unwrap());
//     let r3 = u64::from_le_bytes(slot_hash[16..24].try_into().unwrap());
//     let r4 = u64::from_le_bytes(slot_hash[24..32].try_into().unwrap());
//     let r = r1 ^ r2 ^ r3 ^ r4;
//     // Returns a value in the range [0, 24] inclusive
//     r % 25
// }

#[allow(dead_code)]
async fn simulate_transaction(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) {
    let blockhash = rpc.get_latest_blockhash().await.unwrap();
    let x = rpc
        .simulate_transaction(&Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        ))
        .await;
    println!("Simulation result: {:?}", x);
}

#[allow(dead_code)]
async fn simulate_transaction_with_address_lookup_tables(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
    address_lookup_table_accounts: Vec<AddressLookupTableAccount>,
) {
    let blockhash = rpc.get_latest_blockhash().await.unwrap();
    let tx = VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::V0(
            Message::try_compile(
                &payer.pubkey(),
                instructions,
                &address_lookup_table_accounts,
                blockhash,
            )
            .unwrap(),
        ),
    };
    let s = tx.sanitize();
    println!("Sanitize result: {:?}", s);
    s.unwrap();
    let x = rpc.simulate_transaction(&tx).await;
    println!("Simulation result: {:?}", x);
}

#[allow(unused)]
async fn submit_transaction_batches(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    mut ixs: Vec<solana_sdk::instruction::Instruction>,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(batch_size, ixs.len()))
            .collect::<Vec<Instruction>>();
        submit_transaction_no_confirm(rpc, payer, &batch).await?;
    }
    Ok(())
}

#[allow(unused)]
async fn simulate_transaction_batches(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    mut ixs: Vec<solana_sdk::instruction::Instruction>,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(batch_size, ixs.len()))
            .collect::<Vec<Instruction>>();
        simulate_transaction(rpc, payer, &batch).await;
    }
    Ok(())
}

async fn submit_transaction(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    all_instructions.extend_from_slice(instructions);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    match rpc.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction submitted: {:?}", signature);
            Ok(signature)
        }
        Err(e) => {
            println!("Error submitting transaction: {:?}", e);
            Err(e.into())
        }
    }
}

async fn submit_transaction_no_confirm(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    all_instructions.extend_from_slice(instructions);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    match rpc.send_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction submitted: {:?}", signature);
            Ok(signature)
        }
        Err(e) => {
            println!("Error submitting transaction: {:?}", e);
            Err(e.into())
        }
    }
}

pub async fn get_program_accounts<T>(
    client: &RpcClient,
    program_id: Pubkey,
    filters: Vec<RpcFilterType>,
) -> Result<Vec<(Pubkey, T)>, anyhow::Error>
where
    T: AccountDeserialize + Discriminator + Clone,
{
    let mut all_filters = vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        0,
        &T::discriminator().to_le_bytes(),
    ))];
    all_filters.extend(filters);
    let result = client
        .get_program_accounts_with_config(
            &program_id,
            RpcProgramAccountsConfig {
                filters: Some(all_filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await;

    match result {
        Ok(accounts) => {
            let accounts = accounts
                .into_iter()
                .filter_map(|(pubkey, account)| {
                    if let Ok(account) = T::try_from_bytes(&account.data) {
                        Some((pubkey, account.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            Ok(accounts)
        }
        Err(err) => match err.kind {
            ClientErrorKind::Reqwest(err) => {
                if let Some(status_code) = err.status() {
                    if status_code == StatusCode::GONE {
                        panic!(
                                "\n{} Your RPC provider does not support the getProgramAccounts endpoint, needed to execute this command. Please use a different RPC provider.\n",
                                "ERROR"
                            );
                    }
                }
                return Err(anyhow::anyhow!("Failed to get program accounts: {}", err));
            }
            _ => return Err(anyhow::anyhow!("Failed to get program accounts: {}", err)),
        },
    }
}

// ============ v0.2 Skill System CLI ============

/// Submit a prediction for the winning square.
/// Usage: COMMAND=predict SQUARE=<0-24> cargo run -p skill-cli
async fn predict(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    // Read the predicted square from environment variable
    let square: u8 = std::env::var("SQUARE")
        .expect("Missing SQUARE env var (0-24)")
        .parse()
        .expect("SQUARE must be a number 0-24");

    if square > 24 {
        return Err(anyhow::anyhow!("SQUARE must be 0-24, got {}", square));
    }

    // Get current board state
    let board = get_board(rpc).await?;
    println!("Submitting prediction for round {}", board.round_id);
    println!("Predicted square: {}", square);

    // Build and submit transaction
    let ix = skill_api::sdk::submit_prediction(payer.pubkey(), square);
    let sig = submit_transaction(rpc, payer, &[ix]).await?;

    println!();
    println!("Prediction submitted!");
    println!("Transaction: {}", sig);

    Ok(())
}

/// Display skill statistics for a miner.
/// Usage: COMMAND=skill cargo run -p skill-cli
async fn log_skill(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    // Get miner account
    let authority = std::env::var("AUTHORITY")
        .map(|s| Pubkey::from_str(&s).expect("Invalid AUTHORITY"))
        .unwrap_or(payer.pubkey());

    let miner = get_miner(rpc, authority).await?;

    // Calculate skill multiplier
    let multiplier = miner.calculate_skill_multiplier();
    let multiplier_display = multiplier as f64 / 100.0;

    println!();
    println!("Skill Statistics for {}", authority);
    println!("====================================");
    println!("  Skill Score:      {}", miner.skill_score);
    println!("  Current Streak:   {}", miner.streak);
    println!("  Skill Multiplier: {:.2}x", multiplier_display);
    println!();
    println!("Challenge Stats:");
    println!("  Total Attempts:   {}", miner.challenge_count);
    println!("  Total Wins:       {}", miner.challenge_wins);
    if miner.challenge_count > 0 {
        let win_rate = (miner.challenge_wins as f64 / miner.challenge_count as f64) * 100.0;
        println!("  Win Rate:         {:.1}%", win_rate);
    }
    println!();
    println!("Current Prediction:");
    if miner.prediction == Miner::NO_PREDICTION {
        println!("  None (use COMMAND=predict SQUARE=<0-24> to submit)");
    } else {
        println!("  Square: {} (for round {})", miner.prediction, miner.last_prediction_round);
    }

    Ok(())
}
