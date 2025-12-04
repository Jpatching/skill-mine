use skill_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

/// Allows a miner to submit their prediction for the winning square.
/// Must be called before the round ends.
pub fn process_submit_prediction(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse instruction data
    let args = SubmitPrediction::try_from_bytes(data)?;
    let predicted_square = args.square;

    // Validate prediction is in valid range (0-24)
    if predicted_square > 24 {
        sol_log("Invalid prediction: square must be 0-24");
        return Err(ProgramError::InvalidArgument);
    }

    // Load accounts
    let [signer_info, miner_info, board_info] = accounts else {
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

    // Get current round from board
    let current_round = board.round_id;

    // Check if miner already made a prediction for this round
    if miner.has_prediction_for_round(current_round) {
        sol_log("Already submitted prediction for this round");
        return Err(ProgramError::InvalidAccountData);
    }

    // Submit the prediction
    miner.submit_prediction(predicted_square, current_round);

    sol_log(&format!(
        "Prediction submitted: square {} for round {}",
        predicted_square, current_round
    ));

    Ok(())
}
