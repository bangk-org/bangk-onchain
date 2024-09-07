// File: bangk/src/utils/tokens.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/utils/tokens.rs
// Project: bangk-onchain
// Creation date: Friday 22 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 15 July 2024 @ 14:41:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{debug, Error};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, pubkey::Pubkey,
};
use spl_token_2022::{
    extension::StateWithExtensions,
    instruction::{burn as spl_burn, freeze_account, mint_to, thaw_account, transfer_checked},
    state::{Account, AccountState},
};

use crate::{
    check_mint_ata,
    state::{get_state, mints::BangkMint, stable::StableMint},
    utils::accounts::close_account,
};

/// Determine which accounts are to be frozen/thawed.
#[derive(Clone, Copy)]
pub enum FreezeCheck {
    /// Both accounts should be frozen and thawed.
    Both,
    // Only the payer's account should be thawed and frozen.
    // Payer,
    /// Only the payee's account should be thawed and frozen.
    Payee,
    /// Neither accounts should be frozen or thawed.
    Neither,
}

/// Mint tokens to an account.
///
/// # Parameters
/// * `account` - Account to mint tokens to,
/// * `mint` - Mint handling the tokens,
/// * `signer` - Transaction signer / fee payer,
/// * `program_spl2022` - SPL 2022 Token program account,
/// * `amount` - Amount to mint.
///
/// # Errors
/// If the tokens could not be minted.
pub fn mint<'a>(
    account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    program_spl2022: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    debug!(
        "Minting {} tokens from {} to {}",
        amount, mint.key, account.key
    );

    if mint.lamports() == 0 {
        return Err(Error::UnknownCurrency.into());
    }

    if *mint.key != get_associated_mint(account)? {
        msg!(
            "Account {} is not associated to the mint {}: cannot mint",
            account.key,
            mint.key
        );
        return Err(Error::MismatchATAMint.into());
    }

    let base_state = StateWithExtensions::<Account>::unpack(&account.data.borrow())?
        .base
        .state;
    if base_state == AccountState::Frozen {
        thaw(program_spl2022, &[account], mint, signer)?;
    }
    invoke(
        &mint_to(
            &spl_token_2022::id(),
            mint.key,
            account.key,
            signer.key,
            &[signer.key],
            amount,
        )?,
        &[mint.clone(), account.clone(), signer.clone()],
    )?;
    if base_state == AccountState::Frozen {
        freeze(program_spl2022, &[account], mint, signer)?;
    }
    Ok(())
}

/// Burns tokens from an account.
///
/// # Parameters
/// * `account` - Account to mint tokens to,
/// * `mint` - Mint handling the tokens,
/// * `signer` - Transaction signer / fee payer,
/// * `program_spl2022` - SPL 2022 Token program account,
/// * `amount` - Amount to burn,
/// * `close` - If true and there are no token left on the account, close it.
///
/// # Errors
/// If the tokens could not be burned.
pub fn burn<'a>(
    account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    program_spl2022: &AccountInfo<'a>,
    amount: u64,
    close: bool,
) -> ProgramResult {
    if mint.lamports() == 0 {
        return Err(Error::UnknownCurrency.into());
    }

    if *mint.key != get_associated_mint(account)? {
        msg!(
            "Account {} is not associated to the mint {}: cannot burn",
            account.key,
            mint.key
        );
        return Err(Error::MismatchATAMint.into());
    }

    // If there's no tokens to burn, just skip.
    let balance = get_token_amount(account)?;
    if amount == 0 && balance > 0 {
        msg!("Cannot burn 0 tokens");
        return Err(Error::InvalidAmount.into());
    }
    if balance == 0 && close {
        debug!("Closing account");
        return close_account(signer, account, program_spl2022);
    }

    if amount > balance {
        return Err(Error::InsufficientFunds.into());
    }

    let base_state = StateWithExtensions::<Account>::unpack(&account.data.borrow())?
        .base
        .state;
    if base_state == AccountState::Frozen {
        thaw(program_spl2022, &[account], mint, signer)?;
    }

    debug!("Burning from account");
    invoke(
        &spl_burn(
            &spl_token_2022::id(),
            account.key,
            mint.key,
            signer.key,
            &[signer.key],
            amount,
        )?,
        &[mint.clone(), account.clone(), signer.clone()],
    )?;

    if close && get_token_amount(account)? == 0 {
        close_account(signer, account, program_spl2022)?;
    } else if base_state == AccountState::Frozen {
        freeze(program_spl2022, &[account], mint, signer)?;
    } else {
        // nothing left to do.
    }
    Ok(())
}

/// Transfers tokens from one account to another.
///
/// # Parameters
/// * `source` - Source account,
/// * `target` - Target account,
/// * `mint` - Mint the accounts are associated to,
/// * `signer` - Signer for the transaction,
/// * `program_spl2022` - SPL 2022 Token Program account,
/// * `check` - Determines which accounts (if any) should be frozen / thawed,
/// * `amount` - Amount of tokens to transfer.
///
/// # Errors
/// If the tokens could not be transferred.
pub fn transfer<'a>(
    source: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    program_spl2022: &AccountInfo<'a>,
    check: FreezeCheck,
    amount: u64,
) -> ProgramResult {
    if mint.lamports() == 0 {
        return Err(Error::UnknownCurrency.into());
    }

    if amount == 0 {
        msg!("Impossible to transfer 0 tokens");
        return Err(Error::InvalidAmount.into());
    }

    if get_token_amount(source)? < amount {
        return Err(Error::InsufficientFunds.into());
    }

    match check {
        FreezeCheck::Both => {
            thaw(program_spl2022, &[source, target], mint, signer)?;
            debug!("Transferring tokens");
            execute_transfer(source, target, mint, signer, amount)?;
            freeze(program_spl2022, &[source, target], mint, signer)
        }
        // FreezeCheck::Payer => {
        //     thaw(program_spl2022, &[from], mint, signer)?;
        //     debug!("Transferring tokens");
        //     execute_transfer(from, to, mint, signer, amount)?;
        //     freeze(program_spl2022, &[from], mint, signer)
        // }
        FreezeCheck::Payee => {
            thaw(program_spl2022, &[target], mint, signer)?;
            debug!("Transferring tokens");
            execute_transfer(source, target, mint, signer, amount)?;
            freeze(program_spl2022, &[target], mint, signer)
        }
        FreezeCheck::Neither => execute_transfer(source, target, mint, signer, amount),
    }
}

/// Determines if the amount given in an exchange
/// is the amount to be exchanged or the one exchanged from.
///
/// Due to possible rounding errors, only one of the two is absolute,
/// the other one being rounded. In most cases, this will be the amount
/// received (such that the amount paid will be computed from the amount received
/// and the exchange rate, then rounded up), in some cases - mostly when
/// a project pays dividends or reimburses investors - it's the amount received
/// that is computed based on the amount paid and the exchange rate, then rounded down.
#[derive(Clone, Copy)]
pub enum ExchangeTrueValue {
    /// The amount received is computed (project case).
    Source,
    /// The amount paid is computed.
    Target,
}

/// Transfers tokens from one account to another.
///
/// # Parameters
/// * `source` - Source account.
/// * `target` - Target account.
/// * `mint_source` - Mint of the source currency,
/// * `mint_target` - Mint of the target currency,
/// * `exchange_source` - Bangk's ATA for the source currency,
/// * `exchange_target` - Bangk's ATA for the target currency,
/// * `signer` - Signer for the transaction,
/// * `amount` - Amount of tokens to transfer,
/// * `exchange_rate` - Exchange rate between source and target currency,
/// * `exchange_config` - Determines which of the amount received or paid is to be computed.
///
/// # Errors
/// If the tokens could not be exchanged.
#[allow(clippy::too_many_arguments)]
pub fn transfer_with_exchange<'a>(
    source: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    mint_source: &AccountInfo<'a>,
    mint_target: &AccountInfo<'a>,
    exchange_source: &AccountInfo<'a>,
    exchange_target: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    amount: u64,
    exchange_rate: u64,
    exchange_config: ExchangeTrueValue,
) -> ProgramResult {
    if mint_source.lamports() == 0 {
        return Err(Error::UnknownCurrency.into());
    }

    if mint_target.lamports() == 0 {
        return Err(Error::UnknownCurrency.into());
    }

    if amount == 0 {
        return Err(Error::InvalidAmount.into());
    }

    if exchange_rate == 0 {
        return Err(Error::InvalidExchangeRate.into());
    }

    // Check that the accounts are associated to the correct mints
    check_mint_ata!(mint_source, source, exchange_source);
    check_mint_ata!(mint_target, target, exchange_target);

    if get_token_amount(source)? < amount {
        return Err(Error::InsufficientFunds.into());
    }

    #[allow(clippy::cast_precision_loss)]
    let exchange_rate = exchange_rate as f64 / 1e12_f64;
    let decimals_source = TryInto::<BangkMint<StableMint>>::try_into(mint_source.clone())?
        .state()?
        .decimals;
    let decimals_target = TryInto::<BangkMint<StableMint>>::try_into(mint_target.clone())?
        .state()?
        .decimals;

    let (amount_paid, amount_received) = match exchange_config {
        ExchangeTrueValue::Source => {
            let decimals_ratio = 10_f64.powi(
                i32::from(decimals_target)
                    .checked_sub(i32::from(decimals_source))
                    .ok_or(Error::IntegerOverflow)?,
            );
            let amount_received = (amount as f64 * exchange_rate * decimals_ratio) as u64;
            (amount, amount_received)
        }
        ExchangeTrueValue::Target => {
            let decimals_ratio = 10_f64.powi(
                i32::from(decimals_source)
                    .checked_sub(i32::from(decimals_target))
                    .ok_or(Error::IntegerOverflow)?,
            );
            let amount_paid = (amount as f64 * 1. / exchange_rate * decimals_ratio).ceil() as u64;
            (amount_paid, amount)
        }
    };

    if get_token_amount(exchange_target)? < amount_paid {
        return Err(Error::InsufficientExchangeFunds.into());
    }

    debug!("Transferring tokens to exchange");
    execute_transfer(source, exchange_source, mint_source, signer, amount_paid)?;
    debug!("Transferring tokens from exchange");
    execute_transfer(
        exchange_target,
        target,
        mint_target,
        signer,
        amount_received,
    )?;
    Ok(())
}

/// Transfers tokens from one account to another.
///
/// # Parameters
/// * `source` - Source ATA,
/// * `target` - Target ATA,
/// * `mint` - Mint the ATAs are associated with,
/// * `signer` - Signer (and fee payer) for the transaction,
/// * `amount` - Amount to transfer.
///
/// # Errors
/// If the tokens could not be transferred.
fn execute_transfer<'a>(
    source: &AccountInfo<'a>,
    target: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    invoke(
        &transfer_checked(
            &spl_token_2022::id(),
            source.key,
            mint.key,
            target.key,
            signer.key,
            &[signer.key],
            amount,
            get_state(mint)?.decimals,
        )?,
        &[source.clone(), mint.clone(), target.clone(), signer.clone()],
    )
}

/// Get the mint associated to a token.
///
/// # Parameters
/// * `account` - The token account.
///
/// # Returns
/// * The base state of the mint associated to the account (ATA).
///
/// # Errors
/// If the account's data could no be properly deserialized.
#[inline]
pub fn get_associated_mint(account: &AccountInfo) -> Result<Pubkey, ProgramError> {
    Ok(
        StateWithExtensions::<Account>::unpack(&account.try_borrow_data()?)?
            .base
            .mint,
    )
}

/// Thaws an account.
///
/// # Parameters
/// * `program_spl2022` - SPL 2022 Token Program account.
/// * `accounts` - ATAs to thaw.
/// * `mint` - Mint of the token to thaw.
/// * `signer` - Freeze authority (and fee payer).
///
/// # Errors
/// If the account could not be thawed (no error if it's *already* thawed).
pub fn thaw<'a>(
    program_spl2022: &AccountInfo<'a>,
    accounts: &[&AccountInfo<'a>],
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    debug!("Thawing account");
    accounts
        .iter()
        .filter(|account| {
            let data = &account.data.borrow();
            let Ok(status) = StateWithExtensions::<Account>::unpack(data) else {
                return false;
            };
            status.base.state == AccountState::Frozen
        })
        .try_for_each(|account| {
            invoke(
                &thaw_account(
                    program_spl2022.key,
                    account.key,
                    mint.key,
                    signer.key,
                    &[signer.key],
                )?,
                &[(*account).clone(), mint.clone(), signer.clone()],
            )
        })
}

/// Freezes an account.
///
/// # Parameters
/// * `program_spl2022` - SPL 2022 Token Program account.
/// * `accounts` - Associated Token Accounts to freeze.
/// * `mint` - Mint of the token to thaw.
/// * `signer` - Freeze authority (and fee payer).
///
/// # Errors
/// If the account could not be frozen (no error if it's *already* frozen).
pub fn freeze<'a>(
    program_spl2022: &AccountInfo<'a>,
    accounts: &[&AccountInfo<'a>],
    mint: &AccountInfo<'a>,
    signer: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    debug!("Freezing accounts");
    accounts
        .iter()
        .filter(|account| {
            let data = &account.data.borrow();
            let Ok(status) = StateWithExtensions::<Account>::unpack(data) else {
                return false;
            };
            status.base.state != AccountState::Frozen
        })
        .try_for_each(|account| {
            invoke(
                &freeze_account(
                    program_spl2022.key,
                    account.key,
                    mint.key,
                    signer.key,
                    &[signer.key],
                )?,
                &[(*account).clone(), mint.clone(), signer.clone()],
            )
        })
}

/// Get the amount of (Non-Native) tokens in an account.
///
/// # Parameters
/// * `account` - Account (ATA) to read.
///
/// # Returns
/// * Amount of tokens in the ATA.
///
/// # Errors
/// If the account's data could not be properly deserialized.
#[inline]
pub fn get_token_amount(account: &AccountInfo) -> Result<u64, ProgramError> {
    let state = StateWithExtensions::<Account>::unpack(&account.try_borrow_data()?)?.base;
    Ok(state.amount)
}
