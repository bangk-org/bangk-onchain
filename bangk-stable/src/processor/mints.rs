// File: bangk-stable/src/processor/mints.rs
// Project: bangk-onchain
// Creation date: Sunday 22 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:55:18
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_onchain_common::{
    check_ata_program, check_pda_owner, check_signers, check_spl_program, check_system_program,
    debug,
    pda::BangkPda as _,
    security::{MultiSigPda, MultiSigType, OperationSecurityLevel},
    Error,
};
use borsh::BorshDeserialize as _;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{get_return_data, invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    sysvar::Sysvar as _,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::{
    extension::{metadata_pointer, ExtensionType},
    instruction::{get_account_data_size, initialize_account3, initialize_mint2, mint_to},
    state::Mint,
};
use spl_token_metadata_interface::{
    instruction::{initialize as initialize_metadata, update_field},
    state::{Field, TokenMetadata},
};

use crate::{
    get_token_amount, CreateStableCoinArgs, StableCoinAmountArgs, UpdateStableCoinMetadataArgs,
    EXCHANGE_SEED, STABLE_MINT_SEED,
};

struct MintCreationAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    pda_exchange: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
}

impl<'a> MintCreationAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            mint: next_account_info(accounts_iter)?.clone(),
            pda_exchange: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
        })
    }
}

#[allow(clippy::too_many_lines)]
pub fn mint_creation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &CreateStableCoinArgs,
) -> ProgramResult {
    let ctx = MintCreationAccounts::new(accounts)?;
    msg!("Bangk: creating {} mint", args.currency);

    if ctx.mint.lamports() != 0 {
        msg!("{} already exists", args.currency);
        return Err(Error::UniqueOperationAlreadyExecuted.into());
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);
    check_system_program!(&ctx.program_system);
    check_spl_program!(&ctx.program_token);

    debug!("Initializing mint {}", ctx.mint.key);
    let (mint, mint_bump) = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), args.symbol.as_bytes()],
        &crate::ID,
    );
    let mint_len =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
            .map_err(|_err| Error::CrossProgramCallFailed)?;

    let metadata = TokenMetadata {
        update_authority: Some(*ctx.sig_admin.key).try_into()?,
        mint: *ctx.mint.key,
        name: args.currency.clone(),
        symbol: args.symbol.clone(),
        uri: args.uri.clone(),
        additional_metadata: vec![],
    };

    let meta_len = metadata
        .tlv_size_of()
        .map_err(|_err| Error::InvalidRawData)?;

    let mint_data_len = mint_len
        .checked_add(meta_len)
        .ok_or(Error::IntegerOverflow)?;
    debug!(
        "Creating {} mint's PDA of size {}b.",
        args.currency, mint_data_len
    );

    // Creating the PDA where the mint will be saved
    let mint_rent = Rent::get()?.minimum_balance(mint_data_len);
    debug!("Rent needed: {} lamports", mint_rent);
    let create_pda_instr = create_account(
        ctx.admin1.key,
        ctx.mint.key,
        mint_rent,
        mint_len as u64,
        &spl_token_2022::id(),
    );

    invoke_signed(
        &create_pda_instr,
        &[ctx.admin1.clone(), ctx.mint.clone()],
        &[&[
            STABLE_MINT_SEED.as_bytes(),
            args.symbol.as_bytes(),
            &[mint_bump],
        ]],
    )?;

    debug!("Initializing extensions");
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
    invoke_signed(
        &metadata_pointer::instruction::initialize(
            &spl_token_2022::id(),
            ctx.mint.key,
            Some(*ctx.sig_admin.key),
            Some(*ctx.mint.key),
        )?,
        &[ctx.mint.clone(), ctx.sig_admin.clone()],
        &[admin_seeds.as_slice()],
    )?;

    debug!("Initializing Mint");
    let init_token_mint = initialize_mint2(
        &spl_token_2022::id(),
        ctx.mint.key,
        ctx.sig_admin.key,
        Some(ctx.sig_admin.key),
        args.decimals,
    )?;
    invoke_signed(
        &init_token_mint,
        &[ctx.mint.clone()],
        &[admin_seeds.as_slice()],
    )?;

    debug!("Initializing metadata");
    let init_metadata = initialize_metadata(
        &spl_token_2022::id(),
        ctx.mint.key,
        ctx.sig_admin.key,
        ctx.mint.key,
        ctx.sig_admin.key,
        metadata.name,
        metadata.symbol,
        metadata.uri,
    );

    invoke_signed(
        &init_metadata,
        &[
            ctx.mint.clone(),
            ctx.sig_admin.clone(),
            ctx.sig_admin.clone(),
        ],
        &[admin_seeds.as_slice()],
    )?;

    debug!("Mint successfully initialized");

    debug!("Initializing Bangk Exchange wallet");

    let (pda_exchange, pda_bump) =
        Pubkey::find_program_address(&[EXCHANGE_SEED.as_bytes(), &mint.to_bytes()], &crate::ID);
    if *ctx.pda_exchange.key != pda_exchange {
        msg!("invalid wallet address for exchange wallet");
        return Err(Error::InvalidPdaAddress.into());
    }

    invoke(
        &get_account_data_size(&spl_token_2022::id(), ctx.mint.key, &[])?,
        &[ctx.mint.clone()],
    )?;
    let Some((_key, data_len)) = get_return_data() else {
        msg!("could not retrieve account size");
        return Err(Error::InvalidRawData.into());
    };
    let data_len = u64::try_from_slice(&data_len)?;
    #[allow(clippy::cast_possible_truncation)]
    let wallet_rent = Rent::get()?.minimum_balance(data_len as usize);

    debug!("creating PDA for exchange wallet");
    invoke_signed(
        &create_account(
            ctx.admin1.key,
            ctx.pda_exchange.key,
            wallet_rent,
            data_len,
            &spl_token_2022::id(),
        ),
        &[ctx.admin1.clone(), ctx.pda_exchange.clone()],
        &[&[EXCHANGE_SEED.as_bytes(), &mint.to_bytes(), &[pda_bump]]],
    )?;

    debug!("initializing PDA account");
    invoke_signed(
        &initialize_account3(
            &spl_token_2022::id(),
            ctx.pda_exchange.key,
            ctx.mint.key,
            ctx.sig_admin.key,
        )?,
        &[ctx.pda_exchange.clone(), ctx.mint.clone()],
        &[admin_seeds.as_slice()],
    )?;

    Ok(())
}

struct UpdateCoinMetadataAccounts<'a> {
    _admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
}

impl<'a> UpdateCoinMetadataAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            _admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            mint: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
        })
    }
}

#[allow(clippy::too_many_lines)]
pub fn update_metadata(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &UpdateStableCoinMetadataArgs,
) -> ProgramResult {
    let ctx = UpdateCoinMetadataAccounts::new(accounts)?;
    msg!("Bangk: updating mint {}", ctx.mint.key);

    if ctx.mint.lamports() == 0 {
        msg!("{} does not exist", args.symbol);
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

    if args.currency.is_none() && args.uri.is_none() {
        msg!("The new name and new URI cannot both be empty.");
        return Err(Error::InvalidOperation.into());
    }

    if let Some(currency) = args.currency.clone() {
        invoke_signed(
            &update_field(
                &spl_token_2022::id(),
                ctx.mint.key,
                ctx.sig_admin.key,
                Field::Name,
                currency,
            ),
            &[ctx.mint.clone(), ctx.sig_admin.clone()],
            &[admin_seeds.as_slice()],
        )?;
    }

    if let Some(uri) = args.uri.clone() {
        invoke_signed(
            &update_field(
                &spl_token_2022::id(),
                ctx.mint.key,
                ctx.sig_admin.key,
                Field::Uri,
                uri,
            ),
            &[ctx.mint.clone(), ctx.sig_admin.clone()],
            &[admin_seeds.as_slice()],
        )?;
    }

    Ok(())
}

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
    args: StableCoinAmountArgs,
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

    if args.amount <= 0.0_f64 {
        msg!("Cannot mint a negative or null amount of coins");
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
    let amount = get_token_amount(&ctx.mint, args.amount)?;
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
