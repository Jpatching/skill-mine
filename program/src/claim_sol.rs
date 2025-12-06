use skill_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

/// Claims a block reward.
pub fn process_claim_sol(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, miner_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let miner = miner_info
        .as_account_mut::<Miner>(&skill_api::ID)?
        .assert_mut(|m| m.authority == *signer_info.key)?;
    system_program.is_program(&system_program::ID)?;

    // Normalize amount.
    let amount = miner.claim_sol(&clock);

    sol_log(&format!("Claiming {} SOL", amount as f64 / LAMPORTS_PER_SOL as f64).as_str());

    // Transfer reward to recipient.
    miner_info.send(amount, signer_info);

    Ok(())
}
