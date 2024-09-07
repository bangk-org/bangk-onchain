// File: bangk/src/utils/accounts.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/utils/accounts.rs
// Project: bangk-onchain
// Creation date: Friday 24 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 25 March 2024 @ 16:28:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::debug;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program::invoke};
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token_2022::{
    extension::StateWithExtensions,
    instruction::{close_account as spl_close_account, set_authority, AuthorityType},
    state::{Account, AccountState},
};

use crate::utils::tokens::{freeze, thaw};

/// Create an associate token account.
///
/// # Parameters
/// * `account` - Account to create,
/// * `client` - User owning the ATA,
/// * `mint` - Mint the ATA is associated with,
/// * `signer` - Bangk's ID,
/// * `program_spl2022` - SPL 2022 Token Program account,
/// * `program_system` - System Program account.
///
/// # Errors
/// If the ATA could not be created.
pub fn create_ata<'a>(
    account: &AccountInfo<'a>,
    client: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    program_spl2022: &AccountInfo<'a>,
    program_system: &AccountInfo<'a>,
) -> ProgramResult {
    invoke(
        &create_associated_token_account(signer.key, client.key, mint.key, program_spl2022.key),
        &[
            signer.clone(),
            account.clone(),
            client.clone(),
            mint.clone(),
            program_system.clone(),
            program_spl2022.clone(),
        ],
    )?;

    let base_state = StateWithExtensions::<Account>::unpack(&account.data.borrow())?
        .base
        .state;
    if base_state == AccountState::Frozen {
        thaw(program_spl2022, &[account], mint, signer)?;
    }
    debug!("Adding close authority to token account");
    invoke(
        &set_authority(
            program_spl2022.key,
            account.key,
            Some(signer.key),
            AuthorityType::CloseAccount,
            client.key,
            &[client.key],
        )?,
        &[account.clone(), client.clone()],
    )?;
    if base_state == AccountState::Frozen {
        freeze(program_spl2022, &[account], mint, signer)?;
    }
    Ok(())
}

/// Closes an ATA.
///
/// The ATA is closed and the current Lamport rent is moved to the Bangk wallet.
///
/// # Parameters
/// * `owner` - Owner or signer and fee transaction payer,
/// * `account` - ATA to close,
/// * `token_spl2022` - SPL 2022 Token program.
///
/// # Errors
/// If for some reason the `CPC` fails (there are still tokens on the account?)
pub fn close_account<'a>(
    bangk: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    invoke(
        &spl_close_account(token_program.key, account.key, bangk.key, bangk.key, &[])?,
        &[
            account.clone(),
            bangk.clone(),
            bangk.clone(),
            token_program.clone(),
        ],
    )
}
