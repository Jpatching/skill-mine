use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::{miner_pda, Treasury};

use super::OreAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Miner {
    /// The authority of this miner account.
    pub authority: Pubkey,

    /// The miner's prospects in the current round.
    pub deployed: [u64; 25],

    /// The cumulative amount of SOL deployed on each square prior to this miner's move.
    pub cumulative: [u64; 25],

    /// SOL witheld in reserve to pay for checkpointing.
    pub checkpoint_fee: u64,

    /// The last round that this miner checkpointed.
    pub checkpoint_id: u64,

    /// The last time this miner claimed ORE rewards.
    pub last_claim_ore_at: i64,

    /// The last time this miner claimed SOL rewards.
    pub last_claim_sol_at: i64,

    /// The rewards factor last time rewards were updated on this miner account.
    pub rewards_factor: Numeric,

    /// The amount of SOL this miner can claim.
    pub rewards_sol: u64,

    /// The amount of ORE this miner can claim.
    pub rewards_ore: u64,

    /// The amount of ORE this miner has earned from claim fees.
    pub refined_ore: u64,

    /// The ID of the round this miner last played in.
    pub round_id: u64,

    /// The total amount of SOL this miner has mined across all blocks.
    pub lifetime_rewards_sol: u64,

    /// The total amount of ORE this miner has mined across all blocks.
    pub lifetime_rewards_ore: u64,

    // ============ v0.2 Skill System Fields ============

    /// Cumulative skill points earned from correct predictions (never decreases).
    pub skill_score: u64,

    /// Current round prediction: 0-24 for square, 255 for no prediction.
    pub prediction: u8,

    /// Padding for alignment after u8.
    pub _padding1: [u8; 1],

    /// Consecutive correct predictions (resets on wrong prediction).
    pub streak: u16,

    /// Padding for alignment.
    pub _padding2: [u8; 4],

    /// The round ID when the last prediction was made (anti-replay).
    pub last_prediction_round: u64,

    /// Total number of prediction attempts.
    pub challenge_count: u64,

    /// Total number of correct predictions.
    pub challenge_wins: u64,
}

impl Miner {
    pub fn pda(&self) -> (Pubkey, u8) {
        miner_pda(self.authority)
    }

    pub fn claim_ore(&mut self, clock: &Clock, treasury: &mut Treasury) -> u64 {
        self.update_rewards(treasury);
        let refined_ore = self.refined_ore;
        let rewards_ore = self.rewards_ore;
        let mut amount = refined_ore + rewards_ore;
        self.refined_ore = 0;
        self.rewards_ore = 0;
        treasury.total_unclaimed -= rewards_ore;
        treasury.total_refined -= refined_ore;
        self.last_claim_ore_at = clock.unix_timestamp;

        // Charge a 10% fee and share with miners who haven't claimed yet.
        if treasury.total_unclaimed > 0 {
            let fee = rewards_ore / 10;
            amount -= fee;
            treasury.miner_rewards_factor += Numeric::from_fraction(fee, treasury.total_unclaimed);
            treasury.total_refined += fee;
            self.lifetime_rewards_ore -= fee;
        }

        amount
    }

    pub fn claim_sol(&mut self, clock: &Clock) -> u64 {
        let amount = self.rewards_sol;
        self.rewards_sol = 0;
        self.last_claim_sol_at = clock.unix_timestamp;
        amount
    }

    pub fn update_rewards(&mut self, treasury: &Treasury) {
        // Accumulate rewards, weighted by stake balance.
        if treasury.miner_rewards_factor > self.rewards_factor {
            let accumulated_rewards = treasury.miner_rewards_factor - self.rewards_factor;
            if accumulated_rewards < Numeric::ZERO {
                panic!("Accumulated rewards is negative");
            }
            let personal_rewards = accumulated_rewards * Numeric::from_u64(self.rewards_ore);
            self.refined_ore += personal_rewards.to_u64();
            self.lifetime_rewards_ore += personal_rewards.to_u64();
        }

        // Update this miner account's last seen rewards factor.
        self.rewards_factor = treasury.miner_rewards_factor;
    }

    // ============ v0.2 Skill System Methods ============

    /// No prediction constant (255 means no prediction made).
    pub const NO_PREDICTION: u8 = 255;

    /// Maximum skill multiplier (150 = 1.50x).
    pub const MAX_SKILL_MULTIPLIER: u64 = 150;

    /// Points awarded per correct prediction.
    pub const POINTS_PER_WIN: u64 = 100;

    /// Calculate skill multiplier as percentage (100 = 1.0x, 150 = 1.5x).
    /// Formula: base(100) + log10(score)*5 + streak*2, capped at 150.
    pub fn calculate_skill_multiplier(&self) -> u64 {
        let base = 100u64;

        // Score bonus: +5% per order of magnitude of skill_score
        let score_bonus = if self.skill_score > 0 {
            // Integer approximation of log10
            let log_approx = (64 - self.skill_score.leading_zeros()) as u64 * 3 / 10;
            log_approx.saturating_mul(5)
        } else {
            0
        };

        // Streak bonus: +2% per consecutive win, max 10 streaks = +20%
        let streak_bonus = (self.streak as u64).min(10).saturating_mul(2);

        // Total multiplier, capped at MAX_SKILL_MULTIPLIER
        (base + score_bonus + streak_bonus).min(Self::MAX_SKILL_MULTIPLIER)
    }

    /// Check if miner has made a prediction for a given round.
    pub fn has_prediction_for_round(&self, round_id: u64) -> bool {
        self.last_prediction_round == round_id && self.prediction != Self::NO_PREDICTION
    }

    /// Record a prediction for the current round.
    pub fn submit_prediction(&mut self, square: u8, round_id: u64) {
        self.prediction = square;
        self.last_prediction_round = round_id;
        self.challenge_count += 1;
    }

    /// Evaluate prediction after round ends. Called during checkpoint.
    /// Returns the skill multiplier to apply.
    pub fn evaluate_prediction(&mut self, winning_square: u8, round_id: u64) -> u64 {
        // Only evaluate if prediction was made for this round
        if self.last_prediction_round != round_id || self.prediction == Self::NO_PREDICTION {
            // No prediction made - reset streak but don't penalize score
            self.streak = 0;
            return 100; // 1.0x multiplier
        }

        if self.prediction == winning_square {
            // Correct prediction!
            self.skill_score += Self::POINTS_PER_WIN;
            self.streak += 1;
            self.challenge_wins += 1;
        } else {
            // Wrong prediction - reset streak
            self.streak = 0;
        }

        // Clear prediction for next round
        self.prediction = Self::NO_PREDICTION;

        // Return multiplier to apply to rewards
        self.calculate_skill_multiplier()
    }
}

account!(OreAccount, Miner);
