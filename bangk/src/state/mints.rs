// File: bangk/src/state/mints.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/state/mints.rs
// Project: bangk-onchain
// Creation date: Tuesday 27 February 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:37
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{debug, Error};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    rent::Rent,
    system_instruction::create_account,
    sysvar::Sysvar,
};
use spl_token_2022::{
    extension::{
        default_account_state::instruction::initialize_default_account_state, metadata_pointer,
        ExtensionType,
    },
    instruction::{initialize_mint2, initialize_permanent_delegate},
    state::{AccountState, Mint},
};
use spl_token_metadata_interface::{instruction::initialize, state::Field};

use crate::processor::DELEGATE;

use super::{
    get_mint_metadata, get_state, mint_data::MintData, stable::StableMint, update_metadata_field,
};

/// A Bangk mint.
pub struct BangkMint<'a, T: MintData<'a> = StableMint> {
    /// The content of the mint.
    pub data: T,
    /// Account where the mint is stored on the blockchain.
    pub account: AccountInfo<'a>,
}

impl<'a, T: MintData<'a>> BangkMint<'a, T> {
    /// Creates a new `BangkMint`.
    ///
    /// The mint is **not** yet written on the blockchain, call [`create`](BangkMint::create) for that.
    ///
    /// # Parameters
    /// * `account` - Account where the mint will be stored,
    /// * `data` - Data (and type) of the mint.
    pub fn new(account: &AccountInfo<'a>, data: T) -> Self {
        Self {
            data,
            account: account.clone(),
        }
    }

    /// Writes the mint to the blockchain
    ///
    /// # Parameters
    /// * `payer` - The transaction fee payer,
    /// * `delegate` - The designated delegate,
    /// * `decimals` - The number of decimals the mint will use,
    /// * `default_state` - The default state of the ATAs associated to the mint,
    /// * `bump` - Seed bump used to sign the PDA's creation.
    ///
    /// # Errors
    /// If the mint could not be written to the blockchain.
    pub fn create(
        &self,
        payer: &AccountInfo<'a>,
        delegate: &AccountInfo<'a>,
        decimals: u8,
        default_state: &AccountState,
        bump: u8,
    ) -> ProgramResult {
        debug!("Initializing mint {} (bump = {})", self.account.key, bump);
        let mint_len = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::DefaultAccountState,
            ExtensionType::PermanentDelegate,
            ExtensionType::MetadataPointer,
        ])
        .map_err(|_| Error::CrossProgramCallFailed)?;

        let metadata = self.data.to_metadata(&self.account)?;
        let meta_len = metadata.tlv_size_of().map_err(|_| Error::InvalidRawData)?;

        let data_len = mint_len
            .checked_add(meta_len)
            .ok_or(Error::IntegerOverflow)?;
        debug!(
            "Creating {} mint's PDA of size {}b.",
            metadata.name, data_len
        );

        // Creating the PDA where the mint will be saved
        let rent = Rent::get()?.minimum_balance(data_len);
        debug!("Rent needed: {} lamports", rent);
        let create_pda_instr = create_account(
            payer.key,
            self.account.key,
            rent,
            mint_len as u64,
            &spl_token_2022::id(),
        );

        let mut seeds: Vec<&[u8]> = Vec::new();
        let vec_seeds = self.data.signing_seeds(bump);
        vec_seeds
            .iter()
            .for_each(|seed| seeds.push(seed.as_slice()));
        let seeds = seeds.as_slice();
        invoke_signed(
            &create_pda_instr,
            &[payer.clone(), self.account.clone()],
            &[seeds],
        )?;

        debug!("Initializing extensions");
        invoke(
            &initialize_default_account_state(
                &spl_token_2022::id(),
                self.account.key,
                default_state,
            )?,
            &[self.account.clone()],
        )?;
        invoke(
            &initialize_permanent_delegate(&spl_token_2022::id(), self.account.key, &DELEGATE)?,
            &[self.account.clone()],
        )?;
        invoke(
            &metadata_pointer::instruction::initialize(
                &spl_token_2022::id(),
                self.account.key,
                Some(*payer.key),
                Some(*self.account.key),
            )?,
            &[self.account.clone()],
        )?;

        let init_token_mint = initialize_mint2(
            &spl_token_2022::id(),
            self.account.key,
            delegate.key,
            Some(delegate.key),
            decimals,
        )?;
        invoke(&init_token_mint, &[self.account.clone()])?;

        debug!("Initializing metadata");
        let init_metadata = initialize(
            &spl_token_2022::id(),
            self.account.key,
            delegate.key,
            self.account.key,
            delegate.key,
            metadata.name.clone(),
            metadata.symbol.clone(),
            metadata.uri.clone(),
        );
        invoke(
            &init_metadata,
            &[self.account.clone(), payer.clone(), delegate.clone()],
        )?;

        metadata
            .additional_metadata
            .iter()
            .for_each(|(key, value)| {
                update_metadata_field(delegate, &self.account, Field::Key(key.to_owned()), value)
                    .unwrap_or_default();
            });
        debug!("Mint successfully initialized");
        Ok(())
    }

    /// Update the mint on the blockchain.
    ///
    /// # Parameters
    /// * `payer` - The transaction fee payer.
    ///
    /// # Errors
    /// If the mint could not be updated.
    pub fn update(&self, payer: &AccountInfo<'a>) -> ProgramResult {
        if self.account.lamports() == 0 {
            return Err(Error::UnknownCurrency.into());
        }
        let current_meta = get_mint_metadata(&self.account)?;
        let new_metadata = self.data.to_metadata(&self.account)?;

        if current_meta.name != new_metadata.name {
            update_metadata_field(payer, &self.account, Field::Name, &new_metadata.name)?;
        }

        if current_meta.symbol != new_metadata.symbol {
            update_metadata_field(payer, &self.account, Field::Symbol, &new_metadata.symbol)?;
        }

        if current_meta.uri != new_metadata.uri {
            update_metadata_field(payer, &self.account, Field::Uri, &new_metadata.uri)?;
        }

        for ((ref key, ref old), (_, ref new)) in current_meta
            .additional_metadata
            .iter()
            .zip(new_metadata.additional_metadata.iter())
        {
            debug!("testing update for '{}' (value: '{}')", key, new);
            if old != new {
                update_metadata_field(payer, &self.account, Field::Key(key.to_owned()), new)?;
            }
            debug!("done with '{}'", key);
        }
        debug!("done with update");
        Ok(())
    }

    /// Get the state of the mint as known by the blockchain.
    ///
    /// # Errors
    /// If the state could not be retrieved.
    pub fn state(&self) -> Result<Mint, ProgramError> {
        get_state(&self.account)
    }
}

impl<'a, T> TryFrom<AccountInfo<'a>> for BangkMint<'a, T>
where
    T: MintData<'a> + TryFrom<AccountInfo<'a>>,
    ProgramError: From<<T as TryFrom<AccountInfo<'a>>>::Error>,
{
    type Error = ProgramError;

    fn try_from(value: AccountInfo<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            data: T::try_from(value.clone())?,
            account: value,
        })
    }
}
