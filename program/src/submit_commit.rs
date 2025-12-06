use skill_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

/// Allows a miner to submit a commitment hash for the commit-reveal scheme.
/// Must be called during the commit phase (after deploy phase, before reveal phase).
/// The commitment is: keccak256(square || salt || authority)
pub fn process_submit_commit(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse instruction data
    let args = SubmitCommit::try_from_bytes(data)?;
    let commitment = args.commitment;

    // Validate commitment is non-zero
    if commitment == [0u8; 32] {
        sol_log("Invalid commitment: cannot be all zeros");
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
    round_info.has_seeds(
        &[ROUND, current_round_id.to_le_bytes().as_ref()],
        &skill_api::ID,
    )?;
    let round = round_info.as_account::<Round>(&skill_api::ID)?;

    // Get current slot from clock
    let current_slot = clock.slot;

    // Check we're in commit phase
    if !round.is_commit_phase(current_slot) {
        sol_log(&format!(
            "Not in commit phase. Current slot: {}, commit_start: {}, reveal_start: {}",
            current_slot, round.commit_start_slot, round.reveal_start_slot
        ));
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if miner has deployed SOL this round (stake requirement)
    if miner.round_id != current_round_id {
        sol_log("Must deploy SOL this round before committing");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if miner already committed this round
    if miner.has_commitment_for_round(current_round_id) {
        sol_log("Already submitted commitment for this round");
        return Err(ProgramError::InvalidAccountData);
    }

    // Submit the commitment
    miner.submit_commitment(commitment, current_round_id);

    sol_log(&format!(
        "Commitment submitted for round {}",
        current_round_id
    ));

    Ok(())
}
