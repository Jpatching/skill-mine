use skill_api::prelude::*;
use solana_program::log::sol_log;
use solana_program::program::invoke_signed;
use solana_program::program_pack::Pack;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use steel::*;

/// Initializes the SKILL protocol by creating:
/// - Board PDA (singleton for round tracking)
/// - Config PDA (singleton for admin configuration)
/// - Treasury PDA (singleton for treasury management)
/// - SKILL token mint (with Treasury as mint authority)
/// - Treasury's associated token account for SKILL
pub fn process_initialize(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Initialize::try_from_bytes(data)?;
    let admin = Pubkey::new_from_array(args.admin);
    let fee_collector = Pubkey::new_from_array(args.fee_collector);
    let var_address = Pubkey::new_from_array(args.var_address);

    // Load accounts.
    let [signer_info, board_info, config_info, mint_info, treasury_info, treasury_tokens_info, system_program, token_program, associated_token_program, rent_sysvar] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate signer is authorized admin.
    signer_info.is_signer()?;
    signer_info.has_address(&ADMIN_ADDRESS)?;

    // Validate system programs.
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    rent_sysvar.is_sysvar(&sysvar::rent::ID)?;

    // Validate PDAs are empty (not already initialized).
    board_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[BOARD], &skill_api::ID)?;
    config_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[CONFIG], &skill_api::ID)?;
    treasury_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[TREASURY], &skill_api::ID)?;
    mint_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[MINT], &skill_api::ID)?;

    // Create Board account.
    sol_log("Creating Board account");
    create_program_account::<Board>(
        board_info,
        system_program,
        signer_info,
        &skill_api::ID,
        &[BOARD],
    )?;
    let board = board_info.as_account_mut::<Board>(&skill_api::ID)?;
    board.round_id = 0;
    board.start_slot = u64::MAX;
    board.end_slot = u64::MAX;

    // Create Config account.
    sol_log("Creating Config account");
    create_program_account::<Config>(
        config_info,
        system_program,
        signer_info,
        &skill_api::ID,
        &[CONFIG],
    )?;
    let config = config_info.as_account_mut::<Config>(&skill_api::ID)?;
    config.admin = admin;
    config.bury_authority = admin;
    config.fee_collector = fee_collector;
    config.swap_program = Pubkey::default();
    config.var_address = var_address;
    config.admin_fee = 0;

    // Create Treasury account.
    sol_log("Creating Treasury account");
    create_program_account::<Treasury>(
        treasury_info,
        system_program,
        signer_info,
        &skill_api::ID,
        &[TREASURY],
    )?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&skill_api::ID)?;
    treasury.balance = 0;
    treasury.motherlode = 0;
    treasury.miner_rewards_factor = Numeric::ZERO;
    treasury.stake_rewards_factor = Numeric::ZERO;
    treasury.total_staked = 0;
    treasury.total_unclaimed = 0;
    treasury.total_refined = 0;

    // Create SKILL token mint with Treasury as mint authority.
    sol_log("Creating SKILL mint");

    // Find the bump for the mint PDA
    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[MINT], &skill_api::ID);
    assert_eq!(*mint_info.key, mint_pda, "Mint address mismatch");

    // Calculate rent and allocate
    let rent = Rent::get()?;
    let mint_lamports = rent.minimum_balance(spl_token::state::Mint::LEN);

    // Create account with system program using PDA signer
    invoke_signed(
        &solana_program::system_instruction::create_account(
            signer_info.key,
            mint_info.key,
            mint_lamports,
            spl_token::state::Mint::LEN as u64,
            &spl_token::ID,
        ),
        &[signer_info.clone(), mint_info.clone(), system_program.clone()],
        &[&[MINT, &[mint_bump]]],
    )?;

    // Initialize the mint with Treasury as mint/freeze authority.
    invoke_signed(
        &spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            mint_info.key,
            treasury_info.key,       // mint authority
            Some(treasury_info.key), // freeze authority
            TOKEN_DECIMALS,
        )?,
        &[mint_info.clone()],
        &[],
    )?;

    // Create Treasury's associated token account for SKILL.
    sol_log("Creating Treasury token account");
    create_associated_token_account(
        signer_info,
        treasury_info,
        treasury_tokens_info,
        mint_info,
        system_program,
        token_program,
        associated_token_program,
    )?;

    sol_log("Initialization complete");
    Ok(())
}
