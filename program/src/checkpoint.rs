use skill_api::prelude::*;
use solana_program::{log::sol_log, rent::Rent};
use spl_token::amount_to_ui_amount;
use steel::*;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Checkpoints a miner's rewards.
pub fn process_checkpoint(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, board_info, miner_info, round_info, treasury_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let board = board_info.as_account::<Board>(&skill_api::ID)?;
    let miner = miner_info.as_account_mut::<Miner>(&skill_api::ID)?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&skill_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // If miner has already checkpointed this round, return.
    if miner.checkpoint_id == miner.round_id {
        return Ok(());
    }

    // If round account is empty, verify the correct account was provided.
    // This can happen if the miner attempted to checkpoint after the round expired and the account was closed.
    // In this case, the miner forfeits any potential rewards.
    if round_info.data_is_empty() {
        sol_log(&format!("Round account is empty").as_str());
        round_info.has_seeds(&[ROUND, &miner.round_id.to_le_bytes()], &skill_api::ID)?;
        miner.checkpoint_id = miner.round_id;
        return Ok(());
    }

    // If round is current round, or the miner round ID does not match the provided round, return.
    let round = round_info.as_account_mut::<Round>(&skill_api::ID)?;
    sol_log(&format!("Round ID: {}", round.id).as_str());

    // Check if round is valid and finalized (has slot_hash from reset)
    if round.id == board.round_id || round.id != miner.round_id || !round.is_finalized() {
        sol_log(&format!("Round not valid or not finalized").as_str());
        return Ok(());
    }

    // Ensure round is not expired.
    // In this case, the miner forfeits any potential rewards.
    if clock.slot >= round.expires_at {
        sol_log(&format!("Round expired").as_str());
        miner.checkpoint_id = miner.round_id;
        return Ok(());
    }

    // Calculate bot fee.
    // If the round expires in less than 12h, anyone may checkpoint this account and collect the bot fee.
    let mut bot_fee = 0;
    if clock.slot >= round.expires_at - TWELVE_HOURS_SLOTS {
        bot_fee = miner.checkpoint_fee;
        miner.checkpoint_fee = 0;
    }

    // Calculate miner rewards.
    let mut rewards_sol = 0;
    let mut rewards_ore = 0;
    let mut winning_square_for_skill: Option<u8> = None;

    // Get the winning square (stored directly by Schelling Point logic)
    // and RNG for split/motherlode/top_miner selection
    if let Some(r) = round.rng() {
        // Get the winning square directly (Schelling Point - stored in round)
        let winning_square = round.get_winning_square();
        winning_square_for_skill = Some(winning_square as u8);

        // If the miner deployed to the winning square, calculate rewards.
        if miner.deployed[winning_square] > 0 {
            // Sanity check.
            assert!(
                round.deployed[winning_square] >= miner.deployed[winning_square],
                "Invalid round deployed amount"
            );

            // Calculate SOL rewards.
            let original_deployment = miner.deployed[winning_square];
            let admin_fee = (original_deployment / 100).max(1);
            rewards_sol = original_deployment - admin_fee;
            rewards_sol += ((round.total_winnings as u128 * miner.deployed[winning_square] as u128)
                / round.deployed[winning_square] as u128) as u64;
            sol_log(&format!("Base rewards: {} SOL", rewards_sol as f64 / LAMPORTS_PER_SOL as f64).as_str());

            // Calculate ORE rewards.
            if round.top_miner == SPLIT_ADDRESS {
                // If round is split, split the reward evenly among all miners.
                rewards_ore = ((round.top_miner_reward as u128
                    * miner.deployed[winning_square] as u128)
                    / round.deployed[winning_square] as u128) as u64;
                sol_log(
                    &format!(
                        "Split rewards: {} ORE",
                        amount_to_ui_amount(rewards_ore, TOKEN_DECIMALS)
                    )
                    .as_str(),
                );
            } else {
                // If round is not split, payout to the top miner.
                let top_miner_sample = round.top_miner_sample(r, winning_square);
                if top_miner_sample >= miner.cumulative[winning_square]
                    && top_miner_sample
                        < miner.cumulative[winning_square] + miner.deployed[winning_square]
                {
                    rewards_ore = round.top_miner_reward;
                    round.top_miner = miner.authority;
                    sol_log(
                        &format!(
                            "Top miner rewards: {} ORE",
                            amount_to_ui_amount(rewards_ore, TOKEN_DECIMALS)
                        )
                        .as_str(),
                    );
                }
            }

            // Calculate motherlode rewards.
            if round.motherlode > 0 {
                let motherload_rewards =
                    ((round.motherlode as u128 * miner.deployed[winning_square] as u128)
                        / round.deployed[winning_square] as u128) as u64;
                sol_log(
                    &format!(
                        "Motherlode rewards: {} ORE",
                        amount_to_ui_amount(motherload_rewards, TOKEN_DECIMALS)
                    )
                    .as_str(),
                );
                rewards_ore += motherload_rewards;
            }
        }
    } else {
        // Sanity check.
        // If there is no rng, total deployed should have been reset to zero.
        assert!(
            round.total_deployed == 0,
            "Round total deployed should be zero."
        );

        // Round has no slot hash, refund all SOL.
        let refund_amount = miner.deployed.iter().sum::<u64>();
        sol_log(&format!("Refunding {} SOL", refund_amount as f64 / LAMPORTS_PER_SOL as f64).as_str());
        rewards_sol = refund_amount;
    }

    // Checkpoint rewards.
    miner.update_rewards(treasury);

    // v0.2: Evaluate skill prediction and apply multiplier
    if let Some(winning_square) = winning_square_for_skill {
        let skill_multiplier = miner.evaluate_prediction(winning_square, round.id);
        if skill_multiplier > 100 && rewards_ore > 0 {
            let boosted_ore = (rewards_ore as u128 * skill_multiplier as u128 / 100) as u64;
            let bonus = boosted_ore - rewards_ore;
            sol_log(&format!(
                "Skill bonus: {}x multiplier, +{} ORE",
                skill_multiplier as f64 / 100.0,
                amount_to_ui_amount(bonus, TOKEN_DECIMALS)
            ).as_str());
            rewards_ore = boosted_ore;
        }
    }

    // Checkpoint miner.
    miner.checkpoint_id = round.id;
    miner.rewards_ore += rewards_ore;
    miner.lifetime_rewards_ore += rewards_ore;
    miner.rewards_sol += rewards_sol;
    miner.lifetime_rewards_sol += rewards_sol;

    // Update treasury.
    treasury.total_unclaimed += rewards_ore;

    // Do SOL transfers.
    if rewards_sol > 0 {
        round_info.send(rewards_sol, &miner_info);
    }
    if bot_fee > 0 {
        miner_info.send(bot_fee, &signer_info);
    }

    // Assert miner account has sufficient funds for rent and rewards.
    let account_size = 8 + std::mem::size_of::<Miner>();
    let required_rent = Rent::get()?.minimum_balance(account_size);
    assert!(
        miner_info.lamports() >= required_rent + miner.checkpoint_fee + miner.rewards_sol,
        "Miner does not have sufficient funds for rent and rewards"
    );

    Ok(())
}
