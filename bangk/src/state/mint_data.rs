// File: bangk/src/state/mint_data.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/mint_data.rs
// Project: bangk-onchain
// Creation date: Monday 26 February 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 25 March 2024 @ 16:28:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_token_metadata_interface::state::TokenMetadata;

use crate::processor::BANGK;

/// Common trait all mints must implement.
pub trait MintData<'a> {
    /// Get the root seed used to generate the PDA's address.
    fn base_seed(symbol: &str) -> String;

    /// Get the symbol of the mint's token.
    fn get_symbol(&self) -> String;

    /// Get the seeds used to sign for the mint's PDA.
    #[must_use]
    fn signing_seeds(&self, bump: u8) -> Vec<Vec<u8>> {
        vec![
            Self::base_seed(&self.get_symbol()).into_bytes(),
            BANGK.to_bytes().to_vec(),
            vec![bump],
        ]
    }

    /// Get the Address of the mint for a given symbol.
    ///
    /// This function should not be used by the On Chain program.
    ///
    /// # Parameters
    /// * `symbol` - Symbol of the token.
    ///
    /// # Returns
    /// * Tuple of public Key of the mint and associated bump
    #[must_use]
    fn get_address(symbol: &str) -> (Pubkey, u8) {
        let mut seeds: Vec<&[u8]> = Vec::new();
        let vec_seeds: Vec<Vec<u8>> = vec![
            Self::base_seed(symbol).into_bytes(),
            BANGK.to_bytes().to_vec(),
        ];

        vec_seeds
            .iter()
            .for_each(|seed| seeds.push(seed.as_slice()));
        let seeds = seeds.as_slice();
        Pubkey::find_program_address(seeds, &crate::ID)
    }

    /// Transforms the actual data into `TokenMetadata` that can be written on the blockchain.
    ///
    /// # Parameters
    /// * `account` - Account where the mint is stored.
    ///
    /// # Errors
    /// If the metadata could not be retrieved.
    fn to_metadata(&self, account: &AccountInfo<'a>) -> Result<TokenMetadata, ProgramError>;
}
