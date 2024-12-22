// File: bangk-stable/src/processor/admin.rs
// Project: bangk-onchain
// Creation date: Sunday 22 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:55:18
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::collections::HashSet;

use bangk_onchain_common::{
    check_pda_owner, check_signers, check_system_program, debug,
    pda::BangkPda as _,
    security::{MultiSig, MultiSigPda, MultiSigType, OperationSecurityLevel},
    Error,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{processor::main::INIT_KEY, ConfigurationPda, InitializeArgs, UpdateAdminMultisigArgs};

struct InitializeAccounts<'a> {
    bangk: AccountInfo<'a>,
    config: AccountInfo<'a>,
    admin_sig: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
}

impl<'a> InitializeAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            bangk: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            admin_sig: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

pub fn initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &InitializeArgs,
) -> ProgramResult {
    let ctx = InitializeAccounts::new(accounts)?;
    msg!("Bangk: initializing ICO program");
    check_system_program!(&ctx.program_system);

    if *ctx.bangk.key != INIT_KEY {
        msg!(
            "Signer {} is not authorized to initialize Bangk's ICO program.",
            ctx.bangk.key
        );
        return Err(Error::InvalidSigner.into());
    }

    if ctx.config.lamports() != 0 {
        msg!("program has already been initialized");
        return Err(Error::UniqueOperationAlreadyExecuted.into());
    }

    if [
        args.api_key,
        args.admin1,
        args.admin2,
        args.admin3,
        args.admin4,
    ]
    .iter()
    .collect::<HashSet<_>>()
    .len()
        != 5
    {
        msg!("duplicated key in admin multisig definition");
        return Err(Error::DuplicatedKeyInMultisigDefinition.into());
    }

    // Special case here, we want to make sure there are no risks for double initialization
    let (config_pda, config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (admin_keys_pda, admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);

    if config_pda != *ctx.config.key {
        msg!("invalid configuration PDA");
        return Err(Error::InvalidPdaAddress.into());
    }
    if admin_keys_pda != *ctx.admin_sig.key {
        msg!("invalid multisig PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    // Saving the Configuration PDA on the chain.
    debug!("writing config PDA");
    let config = ConfigurationPda::new(config_bump, ctx.admin_sig.key);
    config.create(&ctx.config, &ctx.bangk, &crate::ID)?;

    // Saving the Admin Keys PDA on the chain
    debug!("writing admin multisig PDA");
    let admin_sig = MultiSig::new(
        MultiSigType::Admin,
        vec![
            args.api_key,
            args.admin1,
            args.admin2,
            args.admin3,
            args.admin4,
        ],
    );
    let pda_admin = MultiSigPda::new(admin_bump, admin_sig);
    pda_admin.create(&ctx.admin_sig, &ctx.bangk, &crate::ID)?;

    msg!("Stable Coins program successfully initialized");
    Ok(())
}

struct UpdateAdminMultisigAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
}

impl<'a> UpdateAdminMultisigAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

pub fn update_admin_multisig(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &UpdateAdminMultisigArgs,
) -> ProgramResult {
    let ctx = UpdateAdminMultisigAccounts::new(accounts)?;
    msg!("Bangk: Updating Admin MultiSig");

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);
    check_system_program!(&ctx.program_system);

    if [
        args.api_key,
        args.admin1,
        args.admin2,
        args.admin3,
        args.admin4,
    ]
    .iter()
    .collect::<HashSet<_>>()
    .len()
        != 5
    {
        msg!("duplicated key in admin multisig definition");
        return Err(Error::DuplicatedKeyInMultisigDefinition.into());
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let mut admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    admin_sig.multisig.keys = vec![
        args.api_key,
        args.admin1,
        args.admin2,
        args.admin3,
        args.admin4,
    ];
    admin_sig.write(&ctx.admin1)
}
