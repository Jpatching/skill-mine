use skill_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

/// Allows a miner to reveal their committed choice.
/// Must be called during the reveal phase (after commit phase, before round ends).
/// Verifies: keccak256(square || salt || authority) == commitment
pub fn process_reveal_choice(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse instruction data
    let args = RevealChoice::try_from_bytes(data)?;
    let square = args.square;
    let salt: [u8; 16] = args.salt;

    // Validate square is in valid range (0-24)
    if square > 24 {
        sol_log("Invalid reveal: square must be 0-24");
        return Err(ProgramError::InvalidArgument);
    }

    // Load accounts
    let clock = Clock::get()?;
    let [signer_info, miner_info, board_info, round_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Validate signer
    signer_info.is_signer()?;

    // Validate miner account
    miner_info
        .is_writable()?
        .has_seeds(&[MINER, signer_info.key.as_ref()], &skill_api::ID)?;

    // Validate board account
    board_info.has_seeds(&[BOARD], &skill_api::ID)?;

    // Parse accounts
    let miner = miner_info.as_account_mut::<Miner>(&skill_api::ID)?;
    let board = board_info.as_account::<Board>(&skill_api::ID)?;

    // Get current round
    let current_round_id = board.round_id;

    // Validate round account
    round_info
        .is_writable()?
        .has_seeds(
            &[ROUND, current_round_id.to_le_bytes().as_ref()],
            &skill_api::ID,
        )?;
    let round = round_info.as_account_mut::<Round>(&skill_api::ID)?;

    // Get current slot from clock
    let current_slot = clock.slot;

    // Check we're in reveal phase
    if !round.is_reveal_phase(current_slot) {
        sol_log(&format!(
            "Not in reveal phase. Current slot: {}, reveal_start: {}",
            current_slot, round.reveal_start_slot
        ));
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if miner has a commitment for this round
    if !miner.has_commitment_for_round(current_round_id) {
        sol_log("No commitment found for this round");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if miner already revealed this round
    if miner.has_revealed_for_round(current_round_id) {
        sol_log("Already revealed for this round");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify the commitment hash matches
    if !miner.verify_commitment(square, &salt) {
        sol_log("Commitment verification failed: hash does not match");
        return Err(ProgramError::InvalidArgument);
    }

    // Record the reveal
    miner.reveal_choice(square, salt);

    // Update round's revealed_count
    round.revealed_count[square as usize] += 1;
    round.total_reveals += 1;

    sol_log(&format!(
        "Choice revealed: square {} for round {} (total reveals: {})",
        square, current_round_id, round.total_reveals
    ));

    Ok(())
}
