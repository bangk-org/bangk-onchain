// File: bangk-stable/src/processor/transfers.rs
// Project: bangk-onchain
// Creation date: Monday 23 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Monday 30 December 2024 @ 16:53:40
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{
    check_ata_exists, check_ata_owner, check_mint_ata, check_pda_owner, check_signers,
    check_spl_program, check_system_program, debug,
    pda::BangkPda as _,
    security::{MultiSigPda, MultiSigType, OperationSecurityLevel},
    Error,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token_2022::instruction::transfer_checked;

use crate::{
    compute_token_amount, get_decimals,
    support::{get_token_amount, is_account_frozen},
    CoinsAmountArgs, ConfigurationPda, UpdateExchangeRatesArgs,
};

struct UpdateExchangeRatesAccounts<'a> {
    admin1: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
}

impl<'a> UpdateExchangeRatesAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Update the exchange rates in the configuration PDA
pub fn update_exchange_rates(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &UpdateExchangeRatesArgs,
) -> ProgramResult {
    let ctx = UpdateExchangeRatesAccounts::new(accounts)?;
    msg!("Bangk: Updating Exchange Rates");

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Routine);
    check_system_program!(&ctx.program_system);

    // Basic integrity check on the rates (none must be negative or null)
    if args.exchange_rates.values().any(|&val| val <= 0.0_f64) {
        msg!("Negative or null exchange rate detected: aborting");
        return Err(Error::InvalidExchangeRate.into());
    }

    ConfigurationPda::check_address(program_id, &ctx.config)?;
    let mut config = ConfigurationPda::from_account(&ctx.config)?;
    config.exchange_rates.clone_from(&args.exchange_rates);
    config.write(&ctx.admin1)
}

struct TransferAccounts<'a> {
    signer: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    source_ata: AccountInfo<'a>,
    target_ata: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
}

impl<'a> TransferAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            signer: next_account_info(accounts_iter)?.clone(),
            mint: next_account_info(accounts_iter)?.clone(),
            source_ata: next_account_info(accounts_iter)?.clone(),
            target_ata: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Transfer tokens from one account to another
pub fn transfer(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CoinsAmountArgs,
) -> ProgramResult {
    let ctx = TransferAccounts::new(accounts)?;
    msg!("Bangk: Transferring Stable Coins");

    check_ata_exists!(&ctx.source_ata, &ctx.target_ata);
    check_ata_owner!(&ctx.signer, &ctx.source_ata);
    check_mint_ata!(&ctx.mint, &ctx.source_ata, &ctx.target_ata);
    check_system_program!(&ctx.program_system);
    check_spl_program!(&ctx.program_token);

    if (ctx.source_ata.lamports() > 0 && is_account_frozen(&ctx.source_ata)?)
        || (ctx.target_ata.lamports() > 0 && is_account_frozen(&ctx.target_ata)?)
    {
        msg!("an account is frozen, aborting");
        return Err(Error::InvalidFreezeStatus.into());
    }

    if args.amount <= 0.0_f64 {
        msg!("Cannot transfer a negative or null amount of coins");
        return Err(Error::InvalidAmount.into());
    }

    let amount = compute_token_amount(&ctx.mint, args.amount)?;
    invoke(
        &transfer_checked(
            &spl_token_2022::id(),
            ctx.source_ata.key,
            ctx.mint.key,
            ctx.target_ata.key,
            ctx.signer.key,
            &[],
            amount,
            get_decimals(&ctx.mint)?,
        )?,
        &[
            ctx.source_ata.clone(),
            ctx.mint.clone(),
            ctx.target_ata.clone(),
            ctx.signer.clone(),
        ],
    )
}

struct ExchangeAccounts<'a> {
    signer: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    source_mint: AccountInfo<'a>,
    target_mint: AccountInfo<'a>,
    source_exchange: AccountInfo<'a>,
    target_exchange: AccountInfo<'a>,
    source_ata: AccountInfo<'a>,
    target_ata: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
}

impl<'a> ExchangeAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            signer: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            source_mint: next_account_info(accounts_iter)?.clone(),
            target_mint: next_account_info(accounts_iter)?.clone(),
            source_exchange: next_account_info(accounts_iter)?.clone(),
            target_exchange: next_account_info(accounts_iter)?.clone(),
            source_ata: next_account_info(accounts_iter)?.clone(),
            target_ata: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Transfer tokens from one account to another
pub fn exchange(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CoinsAmountArgs,
) -> ProgramResult {
    let ctx = ExchangeAccounts::new(accounts)?;
    msg!("Bangk: Exchanging Stable Coins");

    check_pda_owner!(program_id, ctx.config);
    check_ata_exists!(
        &ctx.source_ata,
        &ctx.target_ata,
        &ctx.source_exchange,
        &ctx.target_exchange
    );
    check_ata_owner!(&ctx.signer, &ctx.source_ata);
    check_mint_ata!(&ctx.source_mint, &ctx.source_ata, &ctx.source_exchange);
    check_mint_ata!(&ctx.target_mint, &ctx.target_ata, &ctx.target_exchange);
    check_system_program!(&ctx.program_system);
    check_spl_program!(&ctx.program_token);

    if (ctx.source_ata.lamports() > 0 && is_account_frozen(&ctx.source_ata)?)
        || (ctx.target_ata.lamports() > 0 && is_account_frozen(&ctx.target_ata)?)
    {
        msg!("an account is frozen, aborting");
        return Err(Error::InvalidFreezeStatus.into());
    }

    if args.amount <= 0.0_f64 {
        msg!("Cannot exchange a negative or null amount of coins");
        return Err(Error::InvalidAmount.into());
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();

    ConfigurationPda::check_address(program_id, &ctx.config)?;
    let config = ConfigurationPda::from_account(&ctx.config)?;
    let rate = config.get_exchange_rate(ctx.source_mint.key, ctx.target_mint.key)?;
    let amount = compute_token_amount(&ctx.target_mint, args.amount)?;
    let exchanged_amount = 1.0_f64 / rate * args.amount;
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    let exchanged_amount =
        (exchanged_amount * 10_f64.powi(i32::from(get_decimals(&ctx.source_mint)?))).ceil() as u64;

    if amount > get_token_amount(&ctx.target_exchange)? {
        msg!("not enough tokens in the target exchange: aborting");
        return Err(Error::InsufficientExchangeFunds.into());
    }

    // Transfer source stable coins from user wallet to exchange wallet
    debug!("sending from source to exchange");
    invoke(
        &transfer_checked(
            &spl_token_2022::id(),
            ctx.source_ata.key,
            ctx.source_mint.key,
            ctx.source_exchange.key,
            ctx.signer.key,
            &[],
            exchanged_amount,
            get_decimals(&ctx.source_mint)?,
        )?,
        &[
            ctx.source_ata.clone(),
            ctx.source_mint.clone(),
            ctx.source_exchange.clone(),
            ctx.signer.clone(),
        ],
    )?;

    // Transfer target stable coins from exchange wallet to user wallet
    debug!("sending from exchange to target");
    invoke_signed(
        &transfer_checked(
            &spl_token_2022::id(),
            ctx.target_exchange.key,
            ctx.target_mint.key,
            ctx.target_ata.key,
            ctx.sig_admin.key,
            &[],
            amount,
            get_decimals(&ctx.target_mint)?,
        )?,
        &[
            ctx.target_exchange.clone(),
            ctx.target_mint.clone(),
            ctx.target_ata.clone(),
            ctx.sig_admin.clone(),
        ],
        &[admin_seeds.as_slice()],
    )
}
