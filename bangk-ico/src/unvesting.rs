// File: bangk-ico/src/unvesting.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::Error;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use shank::ShankType;
use solana_program::msg;

/// Definition of the different types of unvesting schemes.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Clone,
    Copy,
    ShankType,
)]
pub enum UnvestingType {
    /// Team Members and Founders
    TeamFounders,
    /// Advisers & Partners
    AdvisersPartners,
    /// Private Sells
    PrivateSells,
    /// Public Sells Week 1-11
    PublicSells1,
    /// Public Sells Week 12-19
    PublicSells2,
    /// Public Sells Week 20-26
    PublicSells3,
}

/// Definition of an unvesting scheme.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ShankType,
    Serialize,
    Deserialize,
)]
pub struct UnvestingScheme {
    /// Type of the unvesting scheme.
    pub kind: UnvestingType,
    /// Start (in weeks) between the launch of BGK and the initial unvesting.
    pub start: u8,
    /// Total duration of the unvesting (weeks).
    pub duration: u8,
    /// Initial unvested amount (x1000 factor).
    pub initial_unvesting: u16,
    /// Weekly unvested amount (x1000 factor).
    pub weekly_unvesting: u16,
    /// Final unvested amount (x1000 factor).
    pub final_unvesting: u16,
}

impl UnvestingScheme {
    /// Checks if an unvesting definition seems valid.
    #[must_use]
    pub fn is_valid(&self) -> Option<bool> {
        // prevent risks of overflow in next computation
        if self.duration < self.start.checked_add(1)? {
            return Some(false);
        }

        let unvest_weeks = u32::from(self.duration.checked_sub(self.start.checked_add(1)?)?);
        let total_weekly = (u32::from(self.weekly_unvesting).checked_mul(unvest_weeks))?;
        let total = u32::from(self.initial_unvesting)
            .checked_add(total_weekly)?
            .checked_add(u32::from(self.final_unvesting))?;
        if self.start == 0
            || self.start > 52
            || self.duration == 0
            || self.duration > 157
            || total != 100 * 1_000_u32
        {
            msg!(
                "unvesting definition invalid: {:?} (total unvested: {})",
                self,
                total
            );
            return Some(false);
        }

        Some(true)
    }

    /// Compute the percentage (as a x1000 factor) of tokens that should be unvested.
    ///
    /// # Parameters
    /// * `launch` - Time at which the BGK token has been launched,
    /// * `now` - Current timestamp.
    ///
    /// # Returns
    /// The percentage of tokens that can be unvested according to the current scheme,
    /// with a x1000 factor applied.
    ///
    /// # Errors
    /// Could happen if for some reason the now date is set before the launch date.
    pub fn unvested(&self, launch: i64, now: i64) -> Result<u64, Error> {
        const WEEK_S: i64 = 86_400 * 7;
        let weeks = u8::try_from(
            (now.checked_sub(launch).ok_or(Error::ArithmeticError)?)
                .checked_div(WEEK_S)
                .ok_or(Error::ArithmeticError)?,
        )
        .map_err(|_err| Error::ArithmeticError)?;
        if weeks < self.start {
            Ok(0_u64)
        } else if weeks >= self.duration {
            Ok(100_000_u64)
        } else {
            let duration = u64::from(
                weeks
                    .checked_sub(self.start)
                    .ok_or(Error::ArithmeticError)?,
            );
            let duration_unvestment = duration
                .checked_mul(u64::from(self.weekly_unvesting))
                .ok_or(Error::ArithmeticError)?;

            Ok(u64::from(self.initial_unvesting)
                .checked_add(duration_unvestment)
                .ok_or(Error::ArithmeticError)?)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::integer_division)]
    #![allow(clippy::unwrap_used)]
    use super::*;

    // That's the max amount of tokens minted, so if this
    // works, the operations will be fine for everyone.
    const NB_TOKENS: u64 = 177_000_000_000_000;

    const fn setup() -> UnvestingScheme {
        UnvestingScheme {
            kind: UnvestingType::TeamFounders,
            start: 52,
            duration: 157,
            initial_unvesting: 10000,
            weekly_unvesting: 800,
            final_unvesting: 6800,
        }
    }

    #[test]
    fn now_before_launch() {
        let scheme = setup();
        let now = -7 * 86_400_i64;
        assert_eq!(scheme.unvested(0, now).unwrap_err(), Error::ArithmeticError);
    }

    #[test]
    fn before_start() {
        let scheme = setup();
        let now = (i64::from(scheme.start) - 1) * 7 * 86_400_i64;
        assert!(scheme.unvested(0, now).is_ok_and(|res| res == 0));
        assert_eq!(scheme.unvested(0, now).unwrap() * NB_TOKENS / 100_000, 0);
    }

    #[test]
    fn initial() {
        let scheme = setup();
        let now = i64::from(scheme.start) * 7 * 86_400_i64;
        assert!(scheme.unvested(0, now).is_ok_and(|res| res == 10_000_u64));
        assert_eq!(
            scheme.unvested(0, now).unwrap() * NB_TOKENS / 100_000,
            17_700_000_000_000
        );
    }

    #[test]
    fn one_week_in() {
        let scheme = setup();
        let now = (i64::from(scheme.start) + 1) * 7 * 86_400_i64;
        assert!(scheme.unvested(0, now).is_ok_and(|res| res == 10_800_u64));
        assert_eq!(
            scheme.unvested(0, now).unwrap() * NB_TOKENS / 100_000,
            19_116_000_000_000
        );
    }

    #[test]
    fn last_week() {
        let scheme = setup();
        let now = (i64::from(scheme.duration) - 1) * 7 * 86_400_i64;
        assert!(scheme.unvested(0, now).is_ok_and(|res| res == 93_200_u64));
        assert_eq!(
            scheme.unvested(0, now).unwrap() * NB_TOKENS / 100_000,
            164_964_000_000_000
        );
    }

    #[test]
    fn after_end() {
        let scheme = setup();
        let now = i64::from(scheme.duration) * 7 * 86_400_i64;
        assert!(scheme.unvested(0, now).is_ok_and(|res| res == 100_000_u64));
        assert_eq!(
            scheme.unvested(0, now).unwrap() * NB_TOKENS / 100_000,
            NB_TOKENS
        );
    }
}
