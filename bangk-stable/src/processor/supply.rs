// File: bangk-stable/src/processor/supply.rs
// Project: bangk-onchain
// Creation date: Tuesday 24 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 24 December 2024 @ 18:55:36
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{
    check_ata_owner, check_ata_program, check_mint_ata, check_pda_owner, check_signers,
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
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::instruction::{burn, close_account, mint_to};

use crate::{
    compute_token_amount,
    support::{get_token_amount, is_account_frozen},
    BurnArgs, CoinsAmountArgs, EXCHANGE_WALLET_SEED,
};

struct MintCoinAccounts<'a> {
    admin: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    user: AccountInfo<'a>,
    ata: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
    program_ata: AccountInfo<'a>,
}

impl<'a> MintCoinAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            mint: next_account_info(accounts_iter)?.clone(),
            user: next_account_info(accounts_iter)?.clone(),
            ata: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
            program_ata: next_account_info(accounts_iter)?.clone(),
        })
    }
}

#[allow(clippy::too_many_lines)]
pub fn mint_coin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CoinsAmountArgs,
) -> ProgramResult {
    let ctx = MintCoinAccounts::new(accounts)?;
    msg!("Bangk: minting {} to {}", ctx.mint.key, ctx.user.key);

    if ctx.mint.lamports() == 0 {
        msg!("{} does not exist", ctx.mint.key);
        return Err(Error::InvalidPdaAddress.into());
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Routine);
    check_system_program!(&ctx.program_system);
    check_spl_program!(&ctx.program_token);
    check_ata_program!(&ctx.program_ata);

    if ctx.ata.lamports() > 0 && is_account_frozen(&ctx.ata)? {
        msg!("destination account is frozen, aborting");
        return Err(Error::InvalidFreezeStatus.into());
    }

    let ata = get_associated_token_address_with_program_id(
        ctx.user.key,
        ctx.mint.key,
        &spl_token_2022::id(),
    );
    if ata != *ctx.ata.key {
        msg!(
            "The given ATA ({}) does not match the expected one",
            ctx.ata.key,
        );
        return Err(Error::InvalidAta.into());
    }

    if args.amount < 0.0_f64 {
        msg!("Cannot mint a negative amount of coins");
        return Err(Error::InvalidAmount.into());
    }

    // If the ATA does not exist yet, create it
    if ctx.ata.lamports() == 0 {
        invoke(
            &create_associated_token_account(
                ctx.admin.key,
                ctx.user.key,
                ctx.mint.key,
                &spl_token_2022::id(),
            ),
            &[
                ctx.admin.clone(),
                ctx.ata.clone(),
                ctx.user.clone(),
                ctx.mint.clone(),
                ctx.program_system.clone(),
                ctx.program_token.clone(),
            ],
        )?;
        debug!("{} has been successfully created", ctx.ata.key);
    } else {
        debug!("{} already exists", ctx.ata.key);
    }

    // Compute the amount of tokens to mint
    if args.amount == 0.0_f64 {
        return Ok(());
    }

    let amount = compute_token_amount(&ctx.mint, args.amount)?;
    debug!("number of tokens to mint: {amount}");

    // Mint the tokens
    invoke_signed(
        &mint_to(
            &spl_token_2022::id(),
            ctx.mint.key,
            ctx.ata.key,
            ctx.sig_admin.key,
            &[],
            amount,
        )?,
        &[ctx.mint.clone(), ctx.ata.clone(), ctx.sig_admin.clone()],
        &[admin_seeds.as_slice()],
    )
}

struct MintExchangeCoinAccounts<'a> {
    _admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    exchange_pda: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
}

impl<'a> MintExchangeCoinAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            _admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            mint: next_account_info(accounts_iter)?.clone(),
            exchange_pda: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
        })
    }
}
#[allow(clippy::too_many_lines)]
pub fn mint_exchange_coin(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CoinsAmountArgs,
) -> ProgramResult {
    let ctx = MintExchangeCoinAccounts::new(accounts)?;
    msg!("Bangk: minting {} to exchange", ctx.mint.key);

    if ctx.mint.lamports() == 0 {
        msg!("{} does not exist", ctx.mint.key);
        return Err(Error::InvalidPdaAddress.into());
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);
    check_system_program!(&ctx.program_system);
    check_spl_program!(&ctx.program_token);

    let (pda_exchange, _pda_bump) = Pubkey::find_program_address(
        &[EXCHANGE_WALLET_SEED.as_bytes(), &ctx.mint.key.to_bytes()],
        &crate::ID,
    );
    if *ctx.exchange_pda.key != pda_exchange {
        msg!("invalid wallet address for exchange wallet");
        return Err(Error::InvalidPdaAddress.into());
    }

    if args.amount <= 0.0_f64 {
        msg!("Cannot mint a negative or null amount of coins");
        return Err(Error::InvalidAmount.into());
    }

    // Compute the amount of tokens to mint
    let amount = compute_token_amount(&ctx.mint, args.amount)?;
    debug!("number of tokens to mint: {amount}");

    // Mint the tokens
    invoke_signed(
        &mint_to(
            &spl_token_2022::id(),
            ctx.mint.key,
            ctx.exchange_pda.key,
            ctx.sig_admin.key,
            &[],
            amount,
        )?,
        &[
            ctx.mint.clone(),
            ctx.exchange_pda.clone(),
            ctx.sig_admin.clone(),
        ],
        &[admin_seeds.as_slice()],
    )
}

struct BurnCoinAccounts<'a> {
    signer: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    ata: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
    program_ata: AccountInfo<'a>,
}

impl<'a> BurnCoinAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            signer: next_account_info(accounts_iter)?.clone(),
            destination: next_account_info(accounts_iter)?.clone(),
            mint: next_account_info(accounts_iter)?.clone(),
            ata: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
            program_ata: next_account_info(accounts_iter)?.clone(),
        })
    }
}

#[allow(clippy::too_many_lines)]
pub fn burn_coin(_program_id: &Pubkey, accounts: &[AccountInfo], args: BurnArgs) -> ProgramResult {
    let ctx = BurnCoinAccounts::new(accounts)?;
    msg!("Bangk: burning {} from {}", ctx.mint.key, ctx.signer.key);

    if ctx.mint.lamports() == 0 {
        msg!("{} does not exist", ctx.mint.key);
        return Err(Error::InvalidPdaAddress.into());
    }

    check_ata_owner!(&ctx.signer, &ctx.ata);
    check_mint_ata!(&ctx.mint, &ctx.ata);
    check_system_program!(&ctx.program_system);
    check_spl_program!(&ctx.program_token);
    check_ata_program!(&ctx.program_ata);

    if ctx.ata.lamports() > 0 && is_account_frozen(&ctx.ata)? {
        msg!("account is frozen, aborting");
        return Err(Error::InvalidFreezeStatus.into());
    }

    let ata = get_associated_token_address_with_program_id(
        ctx.signer.key,
        ctx.mint.key,
        &spl_token_2022::id(),
    );
    if ata != *ctx.ata.key {
        msg!(
            "The given ATA ({}) does not match the expected one",
            ctx.ata.key,
        );
        return Err(Error::InvalidAta.into());
    }

    if args.amount <= 0.0_f64 {
        msg!("Cannot burn a negative or null amount of coins");
        return Err(Error::InvalidAmount.into());
    }

    // Compute the amount of tokens to burn
    let amount = compute_token_amount(&ctx.mint, args.amount)?;
    let ata_amount = get_token_amount(&ctx.ata)?;
    if amount > ata_amount {
        msg!("tried to burn more tokens than present in the account: aborting");
        return Err(Error::InvalidAmount.into());
    }
    let close = args.close_empty && amount == ata_amount;

    debug!("number of tokens to burn: {amount}");

    // Burn the tokens
    invoke(
        &burn(
            &spl_token_2022::id(),
            ctx.ata.key,
            ctx.mint.key,
            ctx.signer.key,
            &[],
            amount,
        )?,
        &[ctx.ata.clone(), ctx.mint.clone(), ctx.signer.clone()],
    )?;

    // Close the account if requested (and empty)
    if close {
        invoke(
            &close_account(
                &spl_token_2022::id(),
                ctx.ata.key,
                ctx.destination.key,
                ctx.signer.key,
                &[],
            )?,
            &[ctx.ata.clone(), ctx.destination.clone(), ctx.signer.clone()],
        )?;
    }

    Ok(())
}
