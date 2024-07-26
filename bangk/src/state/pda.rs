// File: bangk/src/state/pda.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/pda.rs
// Project: bangk-onchain
// Creation date: Monday 26 February 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{debug, Error};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction::create_account,
    sysvar::Sysvar,
};

use crate::processor::BANGK;

/// Common properties of a Bangk PDA
pub trait BangkPda<'a>: BorshDeserialize + BorshSerialize {
    /// Get the name of the PDA.
    ///
    /// This will only be used to generate the seeds.
    fn name() -> &'static str;

    /// Get the PDA's bump
    fn get_bump(&self) -> u8;

    /// Checks that a PDA has the expected [`PdaType`](bangk_onchain_common::pda::PdaType)
    fn is_valid(&self) -> bool;

    /// Sets the account on which the PDA is stored.
    fn set_account(&mut self, account: AccountInfo<'a>);

    /// Get the account on which the PDA is stored.
    ///
    ///
    /// # Errors
    /// Errors if for some reason the associated account wasn't set (no way to happen?)
    fn get_account(&self) -> Result<AccountInfo<'a>, ProgramError>;

    /// Get the seeds used to sign the PDA's address.
    fn seeds(&self) -> Vec<Vec<u8>>;

    /// Get the seeds used to generate the PDA's address.
    #[must_use]
    fn _seeds(keys: &[&Pubkey]) -> Vec<Vec<u8>> {
        let mut res = vec![Self::name().as_bytes().to_vec()];
        keys.iter()
            .for_each(|&key| res.push(key.to_bytes().to_vec()));
        res.push(BANGK.to_bytes().to_vec());

        res
    }

    /// Get the PDA's address.
    ///
    /// This function should **not** be used by the On Chain program.
    ///
    /// # Parameters
    /// * `client` - ID of the client,
    /// * `project_symbol` - Symbol of the project.
    ///
    /// # Returns
    /// * Tuple of public Key of the investment record and associated bump
    #[must_use]
    fn get_address(keys: &[&Pubkey]) -> (Pubkey, u8) {
        let vec_seeds = Self::_seeds(keys);
        let mut seeds: Vec<&[u8]> = Vec::new();
        vec_seeds
            .iter()
            .for_each(|seed| seeds.push(seed.as_slice()));
        let seeds = seeds.as_slice();
        Pubkey::find_program_address(seeds, &crate::ID)
    }

    /// creates the PDA on the chain
    ///
    /// # Parameters
    /// * `payer` - The transaction paying account.
    ///
    /// # Errors
    /// If the account couldn't be recovered, the data failed to be
    /// serialized, rent could not be computed, etc.
    fn create(&self, payer: &AccountInfo<'a>) -> ProgramResult {
        let data = borsh::to_vec(self)?;
        // Create the account for the project data
        let rent = Rent::get()?.minimum_balance(data.len());
        debug!("Rent needed: {} lamports", rent);
        let account = self.get_account()?;
        let create_pda_instr = create_account(
            payer.key,
            account.key,
            rent,
            data.len() as u64,
            &crate::id(),
        );

        let mut seeds: Vec<&[u8]> = Vec::new();
        let mut vec_seeds = self.seeds();
        vec_seeds.push(vec![self.get_bump()]);
        vec_seeds
            .iter()
            .for_each(|seed| seeds.push(seed.as_slice()));
        let seeds = seeds.as_slice();
        invoke_signed(
            &create_pda_instr,
            &[payer.clone(), account.clone()],
            &[seeds],
        )?;
        write(&account, data)?;
        Ok(())
    }

    /// Update the PDA's data.
    ///
    /// # Parameters
    /// * `payer` - The transaction paying account.
    ///
    /// # Errors
    /// If the account couldn't be recovered or the PDA failed to be serialized.
    fn save(&self) -> ProgramResult {
        write(
            &self.get_account()?,
            borsh::to_vec(self).map_err(|_| Error::InvalidRawData)?,
        )
    }

    /// Delete the PDA from the chain.
    ///
    /// First the data is set to zero, then the rent exemption
    /// is recovered by transferring back to the payer account
    /// which means the account will be closed.
    ///
    /// # Parameters
    /// * `payer` - The transaction paying account.
    ///
    /// # Errors
    /// If the account couldn't be recovered, data overwrite failed, etc.
    #[allow(clippy::collection_is_never_read)]
    fn delete(&self, payer: &AccountInfo<'a>) -> ProgramResult {
        if self.get_account()?.lamports() == 0 {
            return Ok(());
        }

        // Set data to 0 for added security
        let mut zero_data = (0..self.get_account()?.data_len())
            .map(|_| 0_u8)
            .collect::<Vec<_>>();
        zero_data.swap_with_slice(*self.get_account()?.try_borrow_mut_data()?);

        // Recover rent exemption
        let lamports = payer.lamports();
        **payer.lamports.borrow_mut() = self
            .get_account()?
            .lamports()
            .checked_add(lamports)
            .ok_or(Error::RentExemptionRetrieval)?;
        **self.get_account()?.lamports.borrow_mut() = 0;
        Ok(())
    }
}

/// Write data to an account.
///
/// # Parameters
/// * `account` - PDA to be written to,
/// * `data` - Data to write.
#[allow(clippy::collection_is_never_read)] // false positive
#[inline]
fn write(account: &AccountInfo, data: impl Into<Vec<u8>>) -> ProgramResult {
    let mut account_data = data.into();
    account_data.swap_with_slice(*account.try_borrow_mut_data()?);
    Ok(())
}

/// Loads a PDA data from an account.
///
/// # Parameters
/// * `account` - Account from which to read the data
///
/// # Errors
/// If the given account does not contain the expected data.
pub fn from_account<'a, T>(account: &AccountInfo<'a>) -> Result<T, ProgramError>
where
    T: BorshDeserialize + BangkPda<'a>,
{
    let data = account.try_borrow_data()?;
    let mut res = T::try_from_slice(&data)?;
    if !res.is_valid() {
        return Err(Error::InvalidPdaType.into());
    }
    res.set_account(account.clone());
    Ok(res)
}
