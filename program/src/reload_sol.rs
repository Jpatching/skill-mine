use skill_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Claims a block reward.
pub fn process_reload_sol(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, automation_info, miner_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let automation = automation_info
        .as_account_mut::<Automation>(&skill_api::ID)?
        .assert_mut(|a| a.executor == *signer_info.key)?
        .assert_mut(|a| a.reload > 0)?;
    let miner = miner_info
        .as_account_mut::<Miner>(&skill_api::ID)?
        .assert_mut(|m| m.authority == automation.authority)?;
    system_program.is_program(&system_program::ID)?;

    // Claim sol from the miner.
    let amount = miner.claim_sol(&clock);

    // Increment automation balance.
    automation.balance += amount;

    // Transfer SOL to automation.
    miner_info.send(amount, automation_info);

    // Log
    sol_log(&format!("Reloading {} SOL", amount as f64 / LAMPORTS_PER_SOL as f64).as_str());

    Ok(())
}
