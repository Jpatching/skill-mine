use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::round_pda;

use super::OreAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Round {
    /// The round number.
    pub id: u64,

    /// The amount of SOL deployed in each square.
    pub deployed: [u64; 25],

    /// The hash of the end slot from Solana, used for RNG (split, motherlode, top_miner).
    /// Sampled from SlotHashes sysvar at round end for unpredictability.
    pub slot_hash: [u8; 32],

    /// The count of miners on each square.
    pub count: [u64; 25],

    /// The slot at which claims for this round account end.
    pub expires_at: u64,

    /// The amount of ORE in the motherlode.
    pub motherlode: u64,

    /// The account to which rent should be returned when this account is closed.
    pub rent_payer: Pubkey,

    /// The top miner of the round.
    pub top_miner: Pubkey,

    /// The amount of ORE to distribute to the top miner.
    pub top_miner_reward: u64,

    /// The total amount of SOL deployed in the round.
    pub total_deployed: u64,

    /// The total amount of SOL put in the ORE vault.
    pub total_vaulted: u64,

    /// The total amount of SOL won by miners for the round.
    pub total_winnings: u64,

    /// The winning square index (0-24) determined by Schelling Point (argmax).
    /// Stored directly to avoid square 0 bug with slot_hash == [0;32].
    pub winning_square: u8,

    /// Bonus squares for this round (RNG from previous round's slot_hash).
    /// 3 bonus squares that provide 2x multiplier if they win.
    pub bonus_squares: [u8; 3],

    /// Padding for alignment (4 bytes to align to 8-byte boundary).
    pub _padding: [u8; 4],

    // ============ v0.6 Commit-Reveal Fields ============

    /// End of deploy phase, start of commit phase.
    pub commit_start_slot: u64,

    /// End of commit phase, start of reveal phase.
    pub reveal_start_slot: u64,

    /// Count of revealed choices per square (for popularity/contrarian calculation).
    pub revealed_count: [u64; 25],

    /// Total number of reveals submitted.
    pub total_reveals: u64,
}

impl Round {
    pub fn pda(&self) -> (Pubkey, u8) {
        round_pda(self.id)
    }

    /// Get the winning square directly (Schelling Point design).
    /// This is the square with the most SOL deployed (argmax).
    pub fn get_winning_square(&self) -> usize {
        self.winning_square as usize
    }

    /// Check if round result is valid (has been finalized via reset).
    pub fn is_finalized(&self) -> bool {
        // A finalized round has a non-zero slot_hash (sampled from Solana)
        self.slot_hash != [0; 32]
    }

    /// Get RNG value from slot_hash for split/motherlode/top_miner selection.
    /// Returns None if round hasn't been finalized yet.
    pub fn rng(&self) -> Option<u64> {
        if !self.is_finalized() {
            return None;
        }
        let r1 = u64::from_le_bytes(self.slot_hash[0..8].try_into().unwrap());
        let r2 = u64::from_le_bytes(self.slot_hash[8..16].try_into().unwrap());
        let r3 = u64::from_le_bytes(self.slot_hash[16..24].try_into().unwrap());
        let r4 = u64::from_le_bytes(self.slot_hash[24..32].try_into().unwrap());
        let r = r1 ^ r2 ^ r3 ^ r4;
        Some(r)
    }

    /// Legacy method for backwards compatibility - use get_winning_square() instead.
    #[deprecated(note = "Use get_winning_square() for Schelling Point design")]
    pub fn winning_square(&self, _rng: u64) -> usize {
        self.winning_square as usize
    }

    pub fn top_miner_sample(&self, rng: u64, winning_square: usize) -> u64 {
        if self.deployed[winning_square] == 0 {
            return 0;
        }
        rng.reverse_bits() % self.deployed[winning_square]
    }

    pub fn calculate_total_winnings(&self, winning_square: usize) -> u64 {
        let mut total_winnings = 0;
        for (i, &deployed) in self.deployed.iter().enumerate() {
            if i != winning_square {
                total_winnings += deployed;
            }
        }
        total_winnings
    }

    pub fn is_split_reward(&self, rng: u64) -> bool {
        // One out of four rounds get split rewards.
        let rng = rng.reverse_bits().to_le_bytes();
        let r1 = u16::from_le_bytes(rng[0..2].try_into().unwrap());
        let r2 = u16::from_le_bytes(rng[2..4].try_into().unwrap());
        let r3 = u16::from_le_bytes(rng[4..6].try_into().unwrap());
        let r4 = u16::from_le_bytes(rng[6..8].try_into().unwrap());
        let r = r1 ^ r2 ^ r3 ^ r4;
        r % 2 == 0
    }

    pub fn did_hit_motherlode(&self, rng: u64) -> bool {
        rng.reverse_bits() % 625 == 0
    }

    // ============ v0.6 Commit-Reveal Methods ============

    /// Round timing constants (in slots, ~0.4s each)
    pub const DEPLOY_PHASE_SLOTS: u64 = 60;  // 24 seconds for deploy
    pub const COMMIT_PHASE_SLOTS: u64 = 30;  // 12 seconds for commit
    pub const REVEAL_PHASE_SLOTS: u64 = 30;  // 12 seconds for reveal
    pub const TOTAL_ROUND_SLOTS: u64 = 120;  // 48 seconds total

    /// Check if round is in deploy phase.
    pub fn is_deploy_phase(&self, current_slot: u64) -> bool {
        current_slot < self.commit_start_slot
    }

    /// Check if round is in commit phase.
    pub fn is_commit_phase(&self, current_slot: u64) -> bool {
        current_slot >= self.commit_start_slot && current_slot < self.reveal_start_slot
    }

    /// Check if round is in reveal phase.
    pub fn is_reveal_phase(&self, current_slot: u64) -> bool {
        current_slot >= self.reveal_start_slot
    }

    /// Get the winning square from revealed_count (argmax of reveals).
    /// Falls back to deployed if no reveals (backward compatibility).
    pub fn get_winning_square_from_reveals(&self) -> usize {
        if self.total_reveals == 0 {
            // No reveals - use deployed as fallback
            self.deployed
                .iter()
                .enumerate()
                .max_by(|(i1, v1), (i2, v2)| v1.cmp(v2).then_with(|| i2.cmp(i1)))
                .map(|(i, _)| i)
                .unwrap_or(0)
        } else {
            // Use revealed_count for winner determination
            self.revealed_count
                .iter()
                .enumerate()
                .max_by(|(i1, v1), (i2, v2)| v1.cmp(v2).then_with(|| i2.cmp(i1)))
                .map(|(i, _)| i)
                .unwrap_or(0)
        }
    }

    /// Generate bonus squares from previous round's slot_hash.
    /// Returns 3 unique bonus squares that give 2x multiplier.
    pub fn generate_bonus_squares(slot_hash: &[u8; 32]) -> [u8; 3] {
        let s1 = slot_hash[0] % 25;
        let mut s2 = slot_hash[8] % 25;
        let mut s3 = slot_hash[16] % 25;

        // Ensure unique squares
        if s2 == s1 {
            s2 = (s2 + 1) % 25;
        }
        if s3 == s1 || s3 == s2 {
            s3 = (s3 + 1) % 25;
        }
        if s3 == s1 || s3 == s2 {
            s3 = (s3 + 1) % 25;
        }

        [s1, s2, s3]
    }

    /// Check if a square is a bonus square.
    pub fn is_bonus_square(&self, square: u8) -> bool {
        self.bonus_squares.contains(&square)
    }

    /// Calculate contrarian bonus (100-148 range) based on popularity.
    /// Less popular winning squares get higher bonus.
    pub fn calculate_contrarian_bonus(&self, winning_square: u8) -> u64 {
        if self.total_reveals == 0 {
            return 100; // No bonus if no reveals
        }

        let popularity = self.revealed_count[winning_square as usize];
        let popularity_pct = (popularity * 100) / self.total_reveals.max(1);

        // Less popular = higher bonus
        // 0% popularity = +48% bonus
        // 100% popularity = +0% bonus
        100 + (100u64.saturating_sub(popularity_pct)).min(48)
    }
}

account!(OreAccount, Round);

#[cfg(test)]
mod tests {
    use solana_program::rent::Rent;

    use super::*;

    #[test]
    fn test_rent() {
        let size_of_round = 8 + std::mem::size_of::<Round>();
        let required_rent = Rent::default().minimum_balance(size_of_round);
        println!("required_rent: {}", required_rent);
        assert!(false);
    }
}
