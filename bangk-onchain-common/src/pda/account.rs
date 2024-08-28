// File: bangk-onchain-common/src/pda/account.rs
// Project: bangk-onchain
// Creation date: Thursday 25 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 12:52:36
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::{create_account, transfer},
    sysvar::Sysvar,
};

use crate::{debug, Error};

/// Define the type of account for a PDA.
///
/// This is a security requirement to make sure that a PDA of one type can't be used for
/// something else than it was supposed to be.
#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum PdaType {
    /// Configuration of the program
    ProgramConfiguration,
    /// Defines a list of keys belonging to a `MultiSig`.
    MultiSig,
    /// Record of a user's ICO investment.
    IcoInvestment,
    /// A record of the state of dividends payments for a project.
    ProjectDividendsTracker,
    /// A record for a client's investment.
    UserProjectInvestment,
    /// A `Timelocked` instruction
    TimelockInstruction,
    /// A Bangk internal wallet
    Wallet,
}

/// Common properties of a Bangk PDA
pub trait BangkPda<'a>: BorshDeserialize + BorshSerialize {
    /// The type of the PDA
    const PDA_TYPE: PdaType;

    /// Get the PDA's bump
    fn get_bump(&self) -> u8;

    /// Checks that a PDA has the expected [`PdaType`]
    fn is_valid(&self) -> bool;

    /// Get the seeds used to sign the PDA's address.
    fn seeds(&self) -> Vec<Vec<u8>>;

    /// Get the account on which the PDA is saved.
    ///
    /// # Errors
    /// If the account is not set
    fn get_account(&self) -> Result<&AccountInfo<'a>, crate::Error>;

    /// Update the PDA's data.
    ///
    /// # Parameters
    /// * `payer` - The transaction paying account (used in case `realloc` necessary).
    ///
    /// # Errors
    /// If the account couldn't be recovered or the PDA failed to be serialized.
    fn write(&self, payer: &AccountInfo<'a>) -> ProgramResult {
        let account = self.get_account()?;
        if account.lamports() == 0 {
            return Err(Error::WriteInsteadOfCreatePda.into());
        }
        let mut account_data = borsh::to_vec(self).map_err(|_err| Error::InvalidRawData)?;
        // test if different sizes
        if account_data.len() != account.data_len() {
            let rent = Rent::get()?.minimum_balance(account_data.len());
            if rent > account.lamports() {
                let diff = rent.saturating_sub(account.lamports());
                invoke(
                    &transfer(payer.key, account.key, diff),
                    &[payer.clone(), account.clone()],
                )?;
            }
            account.realloc(account_data.len(), false)?;
        }
        // if different, increase the size of the account
        account_data.swap_with_slice(*account.try_borrow_mut_data()?);
        Ok(())
    }

    /// creates the PDA on the chain
    ///
    /// # Parameters
    /// * `account` - The account where the data will be saved,
    /// * `payer` - The transaction paying account,
    /// * `program_id` - The program owning the PDA.
    ///
    /// # Errors
    /// If the account couldn't be recovered, the data failed to be
    /// serialized, rent could not be computed, etc.
    fn create(
        &self,
        account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        program_id: &Pubkey,
    ) -> ProgramResult {
        // In case there was a mixup in the PDA constructor.
        if !self.is_valid() {
            return Err(Error::InvalidPdaType.into());
        }

        // Compute the rent exemption
        let mut data = borsh::to_vec(self).map_err(|_err| Error::InvalidRawData)?;
        let rent = Rent::get()?.minimum_balance(data.len());
        debug!("Creating PDA. Rent needed: {} lamports", rent);

        // Create the account
        let create_pda_instr =
            create_account(payer.key, account.key, rent, data.len() as u64, program_id);

        let seeds = self.seeds();
        let seeds = seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
        invoke_signed(
            &create_pda_instr,
            &[payer.clone(), account.clone()],
            &[seeds.as_slice()],
        )?;

        // Write the data on the newly created account
        debug!("writing PDA data");
        data.swap_with_slice(*account.try_borrow_mut_data()?);
        Ok(())
    }

    /// Delete the PDA from the chain.
    ///
    /// First the data is set to zero, then the rent exemption
    /// is recovered by transferring back to the payer account
    /// which means the account will be closed.
    ///
    /// # Parameters
    /// * `account` - The account to delete,
    /// * `payer` - The transaction paying account.
    ///
    /// # Errors
    /// If the account couldn't be recovered, data overwrite failed, etc.
    #[allow(clippy::collection_is_never_read)]
    fn delete(&self, payer: &AccountInfo<'a>) -> ProgramResult {
        let account = self.get_account()?;
        if account.lamports() == 0 {
            return Ok(());
        }

        // Set data to 0 for added security
        let mut zero_data = vec![0; account.data_len()];
        zero_data.swap_with_slice(*account.try_borrow_mut_data()?);

        // Recover rent exemption
        let lamports = payer.lamports();
        **payer.lamports.borrow_mut() = account
            .lamports()
            .checked_add(lamports)
            .ok_or(Error::RentExemptionRetrieval)?;
        **account.lamports.borrow_mut() = 0;
        Ok(())
    }
}
