// File: bangk-ico/src/wallets.rs
// Project: bangk-onchain
// Creation date: Wednesday 21 August 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 12:47:50
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::pda::Seed;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Initial amount sent to Bangk internal wallets
/// Which follows the distribution described in the
/// [White Paper](https://bangk.gitbook.io/whitepaper_en)
pub const WALLET_INIT_AMOUNT: [(WalletType, u64); 10] = [
    (WalletType::Community, 7_000_000_u64),
    (WalletType::DeFiIncentives, 15_000_000_u64),
    (WalletType::Foundation, 14_000_000_u64),
    (WalletType::Ico, 50_000_000_u64),
    (WalletType::Liquidity, 20_000_000_u64),
    (WalletType::Marketing, 16_000_000_u64),
    (WalletType::Partners, 8_000_000_u64),
    (WalletType::ResearchDevelopmentFund, 7_000_000_u64),
    (WalletType::Reserve, 30_000_000_u64),
    (WalletType::TeamsAdvisers, 10_000_000_u64),
];

/// Types of BGK wallets owned by Bangk
#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum WalletType {
    /// Used for promotion, events, *etc.*
    Community,
    /// Promote innovative `DeFi` solutions in the ecosystem
    DeFiIncentives,
    /// Fund ethical and responsible initiatives
    Foundation,
    /// The tokens available during the ICO
    Ico,
    /// Ensure smooth operations
    Liquidity,
    /// Communication and awareness raising campaigns
    Marketing,
    /// Support essential contributions
    Partners,
    /// Improve Bangk's services
    ResearchDevelopmentFund,
    /// Ensure token stability or respond to unexpected dynamics
    Reserve,
    /// Reward the team and the advisers
    TeamsAdvisers,
}

impl From<WalletType> for Seed {
    fn from(value: WalletType) -> Self {
        Self::from(value as u8)
    }
}

impl WalletType {
    /// Get the address and bump of the PDA associated with the wallet
    #[must_use]
    pub fn get_pda(self) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"BangkWallet", &[self as u8]], &crate::ID)
    }

    /// Get the seeds of the PDA associated with the wallet
    pub fn get_seeds(self) -> Vec<Vec<u8>> {
        let (_address, bump) = self.get_pda();
        let seeds: Vec<Seed> = vec!["BangkWallet".into(), self.into(), bump.into()];
        seeds.into_iter().map(Into::into).collect()
    }
}
