use skill_api::prelude::*;
use solana_program::{keccak::hashv, log::sol_log};
use steel::*;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Deploys capital to prospect on a square.
pub fn process_deploy(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Deploy::try_from_bytes(data)?;
    let mut amount = u64::from_le_bytes(args.amount);
    let mask = u32::from_le_bytes(args.squares);

    // TODO Need config account...

    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, authority_info, automation_info, board_info, miner_info, round_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    authority_info.is_writable()?;
    automation_info
        .is_writable()?
        .has_seeds(&[AUTOMATION, &authority_info.key.to_bytes()], &skill_api::ID)?;
    // Allow deploy if: round is waiting to start (end_slot == MAX) OR within active round window
    let board = board_info
        .as_account_mut::<Board>(&skill_api::ID)?
        .assert_mut(|b| b.end_slot == u64::MAX || (clock.slot >= b.start_slot && clock.slot < b.end_slot))?;

    round_info.is_writable()?;

    // Create Round account if it doesn't exist (first deploy after init)
    // Also handle v0.5 migration where old rounds had smaller size (560 vs 568 bytes)
    let expected_size = 8 + std::mem::size_of::<Round>();
    let round = if round_info.data_is_empty() {
        // create_program_account validates the PDA seeds
        create_program_account::<Round>(
            round_info,
            system_program,
            signer_info,
            &skill_api::ID,
            &[ROUND, &board.round_id.to_le_bytes()],
        )?;
        let round = round_info.as_account_mut::<Round>(&skill_api::ID)?;
        round.id = board.round_id;
        round.deployed = [0; 25];
        round.slot_hash = [0; 32];
        round.count = [0; 25];
        round.expires_at = u64::MAX;
        round.rent_payer = *signer_info.key;
        round.motherlode = 0;
        round.top_miner = Pubkey::default();
        round.top_miner_reward = 0;
        round.total_deployed = 0;
        round.total_vaulted = 0;
        round.total_winnings = 0;
        round.winning_square = 0;
        round.bonus_squares = [0; 3];
        round._padding = [0; 4];
        // v0.6 commit-reveal fields (slots set when round starts)
        round.commit_start_slot = 0;
        round.reveal_start_slot = 0;
        round.revealed_count = [0; 25];
        round.total_reveals = 0;
        round
    } else if round_info.data_len() < expected_size {
        // v0.5 Migration: Old round account needs reallocation
        // Transfer additional rent from signer BEFORE resizing (using system program)
        let rent = solana_program::rent::Rent::get()?;
        let new_minimum_balance = rent.minimum_balance(expected_size);
        let current_balance = round_info.lamports();
        if current_balance < new_minimum_balance {
            let diff = new_minimum_balance - current_balance;
            // Use collect() to transfer from signer to round via system program
            round_info.collect(diff, signer_info)?;
        }

        // Now resize the account
        round_info.resize(expected_size)?;

        // Initialize new fields with defaults
        let mut data = round_info.try_borrow_mut_data()?;
        // winning_square at offset 560, _padding at 561-567
        if data.len() >= 568 {
            data[560] = 0; // winning_square
            data[561..568].copy_from_slice(&[0; 7]); // _padding
        }
        drop(data);

        round_info
            .as_account_mut::<Round>(&skill_api::ID)?
            .assert_mut(|r| r.id == board.round_id)?
    } else {
        round_info
            .as_account_mut::<Round>(&skill_api::ID)?
            .assert_mut(|r| r.id == board.round_id)?
    };

    miner_info
        .is_writable()?
        .has_seeds(&[MINER, &authority_info.key.to_bytes()], &skill_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // Wait until first deploy to start round.
    // v0.6 Commit-Reveal: deploy(60) -> commit(30) -> reveal(30) = 120 slots total (~48 seconds)
    if board.end_slot == u64::MAX {
        board.start_slot = clock.slot;
        // v0.6: Use Round timing constants for commit-reveal phases
        round.commit_start_slot = board.start_slot + Round::DEPLOY_PHASE_SLOTS;
        round.reveal_start_slot = round.commit_start_slot + Round::COMMIT_PHASE_SLOTS;
        board.end_slot = round.reveal_start_slot + Round::REVEAL_PHASE_SLOTS;
        round.expires_at = board.end_slot + ONE_DAY_SLOTS;
    }

    // Check if signer is the automation executor.
    let automation = if !automation_info.data_is_empty() {
        let automation = automation_info
            .as_account_mut::<Automation>(&skill_api::ID)?
            .assert_mut(|a| a.executor == *signer_info.key)?
            .assert_mut(|a| a.authority == *authority_info.key)?;
        Some(automation)
    } else {
        None
    };

    // Update amount and mask for automation.
    let mut squares = [false; 25];
    if let Some(automation) = &automation {
        // Set amount
        amount = automation.amount;

        // Set squares
        match AutomationStrategy::from_u64(automation.strategy as u64) {
            AutomationStrategy::Preferred => {
                // Preferred automation strategy. Use the miner authority's provided mask.
                for i in 0..25 {
                    squares[i] = (automation.mask & (1 << i)) != 0;
                }
            }
            AutomationStrategy::Random => {
                // Random automation strategy. Generate a random mask based on number of squares user wants to deploy to.
                let num_squares = ((automation.mask & 0xFF) as u64).min(25);
                let r = hashv(&[&automation.authority.to_bytes(), &round.id.to_le_bytes()]).0;
                squares = generate_random_mask(num_squares, &r);
            }
        }
    } else {
        // Convert provided 32-bit mask into array of 25 booleans, where each bit in the mask
        // determines if that square index is selected (true) or not (false)
        for i in 0..25 {
            squares[i] = (mask & (1 << i)) != 0;
        }
    }

    // Open miner account.
    let miner = if miner_info.data_is_empty() {
        create_program_account::<Miner>(
            miner_info,
            system_program,
            signer_info,
            &skill_api::ID,
            &[MINER, &signer_info.key.to_bytes()],
        )?;
        let miner = miner_info.as_account_mut::<Miner>(&skill_api::ID)?;
        miner.authority = *signer_info.key;
        miner.deployed = [0; 25];
        miner.cumulative = [0; 25];
        miner.rewards_sol = 0;
        miner.rewards_ore = 0;
        miner.round_id = 0;
        miner.checkpoint_id = 0;
        miner.lifetime_rewards_sol = 0;
        miner.lifetime_rewards_ore = 0;
        miner
    } else {
        miner_info
            .as_account_mut::<Miner>(&skill_api::ID)?
            .assert_mut(|m| {
                if let Some(automation) = &automation {
                    m.authority == automation.authority
                } else {
                    m.authority == *signer_info.key
                }
            })?
    };

    // Reset miner
    if miner.round_id != round.id {
        // Assert miner has checkpointed prior round.
        assert!(
            miner.checkpoint_id == miner.round_id,
            "Miner has not checkpointed"
        );

        // Reset miner for new round.
        miner.deployed = [0; 25];
        miner.cumulative = round.deployed;
        miner.round_id = round.id;
    }

    // Calculate all deployments.
    let mut total_amount = 0;
    let mut total_squares = 0;
    for (square_id, &should_deploy) in squares.iter().enumerate() {
        // Skip if square index is out of bounds.
        if square_id > 24 {
            break;
        }

        // Skip if square is not deployed to.
        if !should_deploy {
            continue;
        }

        // Skip if miner already deployed to this square.
        if miner.deployed[square_id] > 0 {
            continue;
        }

        // Record cumulative amount.
        miner.cumulative[square_id] = round.deployed[square_id];

        // Update miner
        miner.deployed[square_id] = amount;

        // Update board
        round.deployed[square_id] += amount;
        round.total_deployed += amount;
        round.count[square_id] += 1;

        // Update totals.
        total_amount += amount;
        total_squares += 1;

        // Exit early if automation does not have enough balance for another square.
        if let Some(automation) = &automation {
            if total_amount + automation.fee + amount > automation.balance {
                break;
            }
        }
    }

    // Top up checkpoint fee.
    if miner.checkpoint_fee == 0 {
        miner.checkpoint_fee = CHECKPOINT_FEE;
        miner_info.collect(CHECKPOINT_FEE, &signer_info)?;
    }

    // Transfer SOL.
    if let Some(automation) = automation {
        automation.balance -= total_amount + automation.fee;
        automation_info.send(total_amount, &round_info);
        automation_info.send(automation.fee, &signer_info);

        // Close automation if balance is less than what's required to deploy 1 square.
        if automation.balance < automation.amount + automation.fee {
            automation_info.close(authority_info)?;
        }
    } else {
        round_info.collect(total_amount, &signer_info)?;
    }

    // Log
    sol_log(
        &format!(
            "Round #{}: deploying {} SOL to {} squares",
            round.id,
            amount as f64 / LAMPORTS_PER_SOL as f64,
            total_squares,
        )
        .as_str(),
    );

    Ok(())
}

fn generate_random_mask(num_squares: u64, r: &[u8]) -> [bool; 25] {
    let mut new_mask = [false; 25];
    let mut selected = 0;
    for i in 0..25 {
        let rand_byte = r[i];
        let remaining_needed = num_squares as u64 - selected as u64;
        let remaining_positions = 25 - i;
        if remaining_needed > 0
            && (rand_byte as u64) * (remaining_positions as u64) < (remaining_needed * 256)
        {
            new_mask[i] = true;
            selected += 1;
        }
    }
    new_mask
}
