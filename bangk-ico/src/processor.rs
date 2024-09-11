// File: bangk-ico/src/processor.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 11 September 2024 @ 18:39:19
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::collections::HashSet;

use bangk_onchain_common::{
    check_ata_exists, check_pda_owner, check_signers, debug, get_ata_owner, get_timestamp,
    pda::BangkPda,
    security::{MultiSig, MultiSigPda, MultiSigType, OperationSecurityLevel},
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
use spl_associated_token_account::instruction::create_associated_token_account;
use spl_token_2022::{
    extension::{metadata_pointer, ExtensionType},
    instruction::{
        get_account_data_size, initialize_account3, initialize_mint2, mint_to, set_authority,
        transfer_checked, AuthorityType,
    },
    state::Mint,
};
use spl_token_metadata_interface::{
    instruction::initialize as initialize_metadata, state::TokenMetadata,
};

use crate::{
    config::ConfigurationPda,
    instruction::{
        BangkIcoInstruction, CancelInvestmentArgs, InitializeArgs, LaunchBGKArgs, MintCreationArgs,
        UpdateAdminMultisigArgs, UserInvestmentArgs,
    },
    investment::{Investment, UserInvestment, UserInvestmentPda},
    timelock::{Timelock, TimelockPda},
    unvesting::UnvestingType,
    ExecuteTransferFromInternalWalletArgs, QueueTransferFromInternalWalletArgs, WalletType,
    INITIAL_UNVESTING_CONFIGURATION, WALLET_INIT_AMOUNT,
};

include!(concat!(env!("OUT_DIR"), "/keys.rs"));

const TOTAL_TOKEN_AMOUNT: u64 = 177_000_000;

/// Main processor for the program
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let Ok(payload) = BangkIcoInstruction::try_from_slice(instruction_data) else {
        return Err(ProgramError::InvalidInstructionData);
    };
    match payload {
        BangkIcoInstruction::Initialize(args) => initialize(program_id, accounts, &args),
        BangkIcoInstruction::MintBGK(args) => mint_creation(program_id, accounts, args),
        BangkIcoInstruction::UpdateAdminMultisig(args) => {
            update_admin_multisig(program_id, accounts, args)
        }
        BangkIcoInstruction::UserInvestment(args) => user_investment(program_id, accounts, args),
        BangkIcoInstruction::QueuePostLaunchAdvisersInvestment(args) => {
            queue_post_launch_adivisers_investment(program_id, accounts, args)
        }
        BangkIcoInstruction::ProcessPostLaunchAdvisersInvestment(args) => {
            process_post_launch_adivisers_investment(program_id, accounts, args)
        }
        BangkIcoInstruction::CancelInvestment(args) => {
            cancel_investment(program_id, accounts, args)
        }
        BangkIcoInstruction::LaunchBGK(args) => launch_bgk(program_id, accounts, args),
        BangkIcoInstruction::VestingRelease => vesting_release(program_id, accounts),
        BangkIcoInstruction::QueueTransferFromInternalWallet(args) => {
            queue_transfer_from_reserve(program_id, accounts, args)
        }
        BangkIcoInstruction::ExecuteTransferFromInternalWallet(args) => {
            execute_transfer_from_reserve(program_id, accounts, args)
        }
    }
}

struct InitializeAccounts<'a> {
    bangk: AccountInfo<'a>,
    config: AccountInfo<'a>,
    admin_sig: AccountInfo<'a>,
    timelock: AccountInfo<'a>,
    _system_program: AccountInfo<'a>,
}

impl<'a> InitializeAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            bangk: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            admin_sig: next_account_info(accounts_iter)?.clone(),
            timelock: next_account_info(accounts_iter)?.clone(),
            _system_program: next_account_info(accounts_iter)?.clone(),
        })
    }
}

fn initialize(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &InitializeArgs,
) -> ProgramResult {
    let ctx = InitializeAccounts::new(accounts)?;
    msg!("Bangk: initializing ICO program");

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

    if INITIAL_UNVESTING_CONFIGURATION.len() != 6 {
        msg!(
            "Unvesting definition should have 6 elements (got {})",
            INITIAL_UNVESTING_CONFIGURATION.len()
        );
        return Err(Error::InvalidUnvestingDefinition.into());
    }

    if INITIAL_UNVESTING_CONFIGURATION
        .iter()
        .map(|def| def.kind)
        .collect::<HashSet<_>>()
        .len()
        != 6
    {
        msg!("unvesting definition had a duplicate and missing type");
        return Err(Error::InvalidUnvestingDefinition.into());
    }

    if INITIAL_UNVESTING_CONFIGURATION
        .iter()
        .any(|def| !def.is_valid().is_some_and(|valid| valid))
    {
        return Err(Error::InvalidUnvestingDefinition.into());
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
    let (timelock_pda, timelock_bump) = TimelockPda::get_address(&crate::ID);

    if config_pda != *ctx.config.key {
        msg!("invalid configuration PDA");
        return Err(Error::InvalidPdaAddress.into());
    }
    if admin_keys_pda != *ctx.admin_sig.key {
        msg!("invalid multisig PDA");
        return Err(Error::InvalidPdaAddress.into());
    }
    if timelock_pda != *ctx.timelock.key {
        msg!("invalid timelock PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    // Saving the Configuration PDA on the chain.
    debug!("writing config PDA");
    let config = ConfigurationPda::new(
        config_bump,
        &INITIAL_UNVESTING_CONFIGURATION,
        ctx.admin_sig.key,
    );
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

    // Initializing the timelock PDAs
    let timelock = TimelockPda::new(timelock_bump);
    timelock.create(&ctx.timelock, &ctx.bangk, &crate::ID)?;

    msg!("ICO program successfully initialized");
    Ok(())
}

struct MintCreationAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    pda_community: AccountInfo<'a>,
    pda_defi: AccountInfo<'a>,
    pda_foundation: AccountInfo<'a>,
    pda_ico: AccountInfo<'a>,
    pda_liquidity: AccountInfo<'a>,
    pda_marketing: AccountInfo<'a>,
    pda_partners: AccountInfo<'a>,
    pda_rd: AccountInfo<'a>,
    pda_reserve: AccountInfo<'a>,
    pda_team: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
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
            mint_bgk: next_account_info(accounts_iter)?.clone(),
            pda_community: next_account_info(accounts_iter)?.clone(),
            pda_defi: next_account_info(accounts_iter)?.clone(),
            pda_foundation: next_account_info(accounts_iter)?.clone(),
            pda_ico: next_account_info(accounts_iter)?.clone(),
            pda_liquidity: next_account_info(accounts_iter)?.clone(),
            pda_marketing: next_account_info(accounts_iter)?.clone(),
            pda_partners: next_account_info(accounts_iter)?.clone(),
            pda_rd: next_account_info(accounts_iter)?.clone(),
            pda_reserve: next_account_info(accounts_iter)?.clone(),
            pda_team: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
        })
    }
}

#[allow(clippy::too_many_lines)]
fn mint_creation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MintCreationArgs,
) -> ProgramResult {
    let ctx = MintCreationAccounts::new(accounts)?;
    msg!("Bangk: creating BGK mint");

    if ctx.mint_bgk.lamports() != 0 {
        msg!("program has already been initialized");
        return Err(Error::UniqueOperationAlreadyExecuted.into());
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);

    debug!("Initializing mint {}", ctx.mint_bgk.key);
    let mint_len =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
            .map_err(|_err| Error::CrossProgramCallFailed)?;

    let metadata = TokenMetadata {
        update_authority: Some(*ctx.sig_admin.key).try_into()?,
        mint: *ctx.mint_bgk.key,
        name: "Bangk Coin".to_owned(),
        symbol: "BGK".to_owned(),
        uri: "https://api.bangk.app/token-bgk".to_owned(),
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
        metadata.name, mint_data_len
    );

    // Creating the PDA where the mint will be saved
    let mint_rent = Rent::get()?.minimum_balance(mint_data_len);
    debug!("Rent needed: {} lamports", mint_rent);
    let create_pda_instr = create_account(
        ctx.admin1.key,
        ctx.mint_bgk.key,
        mint_rent,
        mint_len as u64,
        &spl_token_2022::id(),
    );

    invoke_signed(
        &create_pda_instr,
        &[ctx.admin1.clone(), ctx.mint_bgk.clone()],
        &[&[b"Mint", b"BGK", &[args.bump]]],
    )?;

    debug!("Initializing extensions");
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
    invoke_signed(
        &metadata_pointer::instruction::initialize(
            &spl_token_2022::id(),
            ctx.mint_bgk.key,
            Some(*ctx.sig_admin.key),
            Some(*ctx.mint_bgk.key),
        )?,
        &[ctx.mint_bgk.clone(), ctx.sig_admin.clone()],
        &[admin_seeds.as_slice()],
    )?;

    debug!("Initializing Mint");
    let init_token_mint = initialize_mint2(
        &spl_token_2022::id(),
        ctx.mint_bgk.key,
        ctx.sig_admin.key,
        None,
        6,
    )?;
    invoke_signed(
        &init_token_mint,
        &[ctx.mint_bgk.clone()],
        &[admin_seeds.as_slice()],
    )?;

    debug!("Initializing metadata");
    let init_metadata = initialize_metadata(
        &spl_token_2022::id(),
        ctx.mint_bgk.key,
        ctx.sig_admin.key,
        ctx.mint_bgk.key,
        ctx.sig_admin.key,
        metadata.name,
        metadata.symbol,
        metadata.uri,
    );

    invoke_signed(
        &init_metadata,
        &[ctx.mint_bgk.clone(), ctx.sig_admin.clone()],
        &[admin_seeds.as_slice()],
    )?;

    debug!("Mint successfully initialized");

    debug!("Initializing Bangk wallets");
    let mut total: u64 = 0;

    for (wallet, amount) in WALLET_INIT_AMOUNT {
        let pda = match wallet {
            WalletType::Community => &ctx.pda_community,
            WalletType::DeFiIncentives => &ctx.pda_defi,
            WalletType::Foundation => &ctx.pda_foundation,
            WalletType::Ico => &ctx.pda_ico,
            WalletType::Liquidity => &ctx.pda_liquidity,
            WalletType::Marketing => &ctx.pda_marketing,
            WalletType::Partners => &ctx.pda_partners,
            WalletType::ResearchDevelopmentFund => &ctx.pda_rd,
            WalletType::Reserve => &ctx.pda_reserve,
            WalletType::TeamsAdvisers => &ctx.pda_team,
        };
        let address = wallet.get_pda().0;
        if *pda.key != address {
            msg!("invalid wallet address for wallet {:?}", wallet);
            return Err(Error::InvalidPdaAddress.into());
        }

        let wallet_seeds = wallet.get_seeds();
        let wallet_seeds = wallet_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
        invoke(
            &get_account_data_size(&spl_token_2022::id(), ctx.mint_bgk.key, &[])?,
            &[ctx.mint_bgk.clone()],
        )?;
        let Some((_key, data_len)) = get_return_data() else {
            msg!("could not retrieve account size");
            return Err(Error::InvalidRawData.into());
        };
        let data_len = u64::try_from_slice(&data_len)?;
        #[allow(clippy::cast_possible_truncation)]
        let wallet_rent = Rent::get()?.minimum_balance(data_len as usize);

        debug!("creating PDA for wallet {:?}", wallet);
        invoke_signed(
            &create_account(
                ctx.admin1.key,
                pda.key,
                wallet_rent,
                data_len,
                &spl_token_2022::id(),
            ),
            &[ctx.admin1.clone(), pda.clone()],
            &[wallet_seeds.as_slice()],
        )?;

        debug!("initializing PDA account");
        invoke_signed(
            &initialize_account3(
                &spl_token_2022::id(),
                pda.key,
                ctx.mint_bgk.key,
                ctx.sig_admin.key,
            )?,
            &[pda.clone(), ctx.mint_bgk.clone()],
            &[admin_seeds.as_slice()],
        )?;

        debug!("minting {} BGK to wallet {:?}", amount, wallet);
        invoke_signed(
            &mint_to(
                &spl_token_2022::id(),
                ctx.mint_bgk.key,
                pda.key,
                ctx.sig_admin.key,
                &[],
                amount.saturating_mul(1_000_000),
            )?,
            &[ctx.mint_bgk.clone(), pda.clone(), ctx.sig_admin.clone()],
            &[admin_seeds.as_slice()],
        )?;
        total = total.saturating_add(amount);
    }

    // Check that we dispatched the expected amount of tokens
    if total != TOTAL_TOKEN_AMOUNT {
        msg!(
            "minted amount {} does not match the expected amount {}",
            total,
            TOTAL_TOKEN_AMOUNT
        );
        return Err(Error::InvalidAmount.into());
    }

    // Revoking mint authority
    debug!("Revoking mint authority");
    invoke_signed(
        &set_authority(
            ctx.program_token.key,
            ctx.mint_bgk.key,
            None,
            AuthorityType::MintTokens,
            ctx.sig_admin.key,
            &[],
        )?,
        &[ctx.mint_bgk.clone(), ctx.sig_admin.clone()],
        &[admin_seeds.as_slice()],
    )?;

    Ok(())
}

struct UpdateAdminMultisigAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> UpdateAdminMultisigAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

fn update_admin_multisig(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UpdateAdminMultisigArgs,
) -> ProgramResult {
    let ctx = UpdateAdminMultisigAccounts::new(accounts)?;
    msg!("Bangk: Updating Admin MultiSig");

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);

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

struct UserInvestmentAccounts<'a> {
    api: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    investment: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> UserInvestmentAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            api: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            investment: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

fn user_investment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UserInvestmentArgs,
) -> ProgramResult {
    let ctx = UserInvestmentAccounts::new(accounts)?;
    msg!("Bangk: Creating / Updating investment for {}", args.user);

    check_pda_owner!(program_id, ctx.config, ctx.sig_admin, ctx.investment);
    check_signers!(accounts, &ctx.sig_admin);

    // Special case here, we want to make sure there are no risks for the wrong PDA address to be given, so we recompute it
    let (investment_pda, investment_bump) = UserInvestmentPda::get_address(args.user, &crate::ID);
    if investment_pda != *ctx.investment.key {
        msg!("invalid user investment PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    ConfigurationPda::check_address(&crate::ID, &ctx.config)?;
    let mut config = ConfigurationPda::from_account(&ctx.config)?;
    config.amount_invested = config.amount_invested.saturating_add(args.amount);

    let max_amount = WALLET_INIT_AMOUNT
        .iter()
        .find(|(kind, _amount)| *kind == WalletType::Ico)
        .map(|(_kind, amount)| amount.saturating_mul(1_000_000))
        .unwrap_or_default();
    if config.amount_invested > max_amount {
        msg!(
            "the maximum amount of available tokens has been exceeded ({} vs {})",
            config.amount_invested,
            max_amount
        );
        return Err(Error::InvalidAmount.into());
    }

    config.write(&ctx.api)?;

    if config.launch_date > 0 && config.launch_date <= get_timestamp()? {
        if args.invest_kind == UnvestingType::AdvisersPartners {
            msg!("use instruction post_launch_advisers_investment instead");
        }
        return Err(Error::IcoInvestAfterLaunch.into());
    }

    // If PdA doesn't exist yet, create it, otherwise update it
    if ctx.investment.lamports() == 0 {
        let investment =
            UserInvestment::new(args.user, args.invest_kind, args.amount, args.custom_rule)?;
        let pda = UserInvestmentPda::new(investment_bump, investment);
        pda.create(&ctx.investment, &ctx.api, &crate::ID)
    } else {
        UserInvestmentPda::check_address(args.user, &crate::ID, &ctx.investment)?;
        let mut pda = UserInvestmentPda::from_account(&ctx.investment)?;
        pda.investment.investments.push(Investment {
            kind: args.invest_kind,
            timestamp: get_timestamp()?,
            custom_rule: args.custom_rule,
            amount_bought: args.amount,
            amount_released: 0,
        });
        pda.write(&ctx.api)
    }
}

struct QueuePostLaunchInvestmentAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    timelock: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> QueuePostLaunchInvestmentAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            timelock: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

fn queue_post_launch_adivisers_investment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UserInvestmentArgs,
) -> ProgramResult {
    let ctx = QueuePostLaunchInvestmentAccounts::new(accounts)?;
    msg!(
        "Bangk: Queuing post-launch creating / updating investment for {}",
        args.user
    );

    check_pda_owner!(program_id, ctx.config, ctx.sig_admin, ctx.timelock);
    check_signers!(accounts, &ctx.sig_admin);

    ConfigurationPda::check_address(&crate::ID, &ctx.config)?;
    let mut config = ConfigurationPda::from_account(&ctx.config)?;
    if config.launch_date == 0 {
        return Err(Error::PostLaunchInvestmentBeforeLaunch.into());
    }

    config.amount_invested = config.amount_invested.saturating_add(args.amount);

    let max_amount = WALLET_INIT_AMOUNT
        .iter()
        .find(|(kind, _amount)| *kind == WalletType::Ico)
        .map(|(_kind, amount)| amount.saturating_mul(1_000_000))
        .unwrap_or_default();
    if config.amount_invested > max_amount {
        msg!(
            "the maximum amount of available tokens has been exceeded ({} vs {})",
            config.amount_invested,
            max_amount
        );
        return Err(Error::InvalidAmount.into());
    }
    config.write(&ctx.admin1)?;

    // Only advisers & partners can get investments post-launch
    if args.invest_kind != UnvestingType::AdvisersPartners {
        msg!("this operation is only available for advisers & partners investments: aborting");
        return Err(Error::InvalidOperation.into());
    }

    // Create the timelocked instruction
    TimelockPda::check_address(&crate::ID, &ctx.timelock)?;
    let mut timelock_pda = TimelockPda::from_account(&ctx.timelock)?;
    let timelock = Timelock::post_launch_investment(args.user, args.custom_rule, args.amount)?;
    timelock_pda.instructions.push(timelock);
    timelock_pda.write(&ctx.admin1)
}

struct ProcessPostLaunchInvestmentAccounts<'a> {
    payer: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    timelock: AccountInfo<'a>,
    investment: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> ProcessPostLaunchInvestmentAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            payer: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            timelock: next_account_info(accounts_iter)?.clone(),
            investment: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

fn process_post_launch_adivisers_investment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: UserInvestmentArgs,
) -> ProgramResult {
    let ctx = ProcessPostLaunchInvestmentAccounts::new(accounts)?;
    msg!(
        "Bangk: Processing post-launch creating / updating investment for {}",
        args.user
    );

    check_pda_owner!(
        program_id,
        ctx.config,
        ctx.sig_admin,
        ctx.investment,
        ctx.timelock
    );
    check_signers!(accounts, &ctx.sig_admin);

    // Check that there’s a queued transfer, and remove it from the list if found
    TimelockPda::check_address(&crate::ID, &ctx.timelock)?;
    let mut timelock = TimelockPda::from_account(&ctx.timelock)?;
    timelock.process_post_launch_investment(
        &args.user,
        args.custom_rule,
        args.amount,
        &ctx.payer,
    )?;
    debug!("queued operation is ready, proceeding");

    // Special case here, we want to make sure there are no risks for the wrong PDA address to be given, so we recompute it
    let (investment_pda, investment_bump) = UserInvestmentPda::get_address(args.user, &crate::ID);
    if investment_pda != *ctx.investment.key {
        msg!("invalid user investment PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    ConfigurationPda::check_address(&crate::ID, &ctx.config)?;

    // If PdA doesn't exist yet, create it, otherwise update it
    if ctx.investment.lamports() == 0 {
        let investment = UserInvestment::new(
            args.user,
            UnvestingType::AdvisersPartners,
            args.amount,
            args.custom_rule,
        )?;
        let pda = UserInvestmentPda::new(investment_bump, investment);
        pda.create(&ctx.investment, &ctx.payer, &crate::ID)?;
    } else {
        UserInvestmentPda::check_address(args.user, &crate::ID, &ctx.investment)?;
        let mut pda = UserInvestmentPda::from_account(&ctx.investment)?;
        pda.investment.investments.push(Investment {
            kind: UnvestingType::AdvisersPartners,
            timestamp: get_timestamp()?,
            custom_rule: args.custom_rule,
            amount_bought: args.amount,
            amount_released: 0,
        });
        pda.write(&ctx.payer)?;
    }

    Ok(())
}

struct CancelInvestmentAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    investment: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> CancelInvestmentAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            investment: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Cancel a user's investment.
fn cancel_investment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CancelInvestmentArgs,
) -> ProgramResult {
    let ctx = CancelInvestmentAccounts::new(accounts)?;
    msg!("Bangk: deleting investment for {}", args.user);

    check_pda_owner!(program_id, ctx.config, ctx.sig_admin, ctx.investment);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Sensitive);

    ConfigurationPda::check_address(&crate::ID, &ctx.config)?;
    let mut config = ConfigurationPda::from_account(&ctx.config)?;
    config.amount_invested = config.amount_invested.saturating_sub(args.amount);
    config.write(&ctx.admin1)?;

    if config.launch_date > 0 && config.launch_date < get_timestamp()? {
        return Err(Error::CancelIcoInvestmentAfterLaunch.into());
    }

    // If PdA doesn't exist that's an error
    if ctx.investment.lamports() == 0 {
        return Err(Error::InvestmentDoesNotExist.into());
    }

    // Special case here, we want to make sure there are no risks for the wrong PDA address to be given, so we recompute it
    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(args.user, &crate::ID);
    if investment_pda != *ctx.investment.key {
        msg!("invalid user investment PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    UserInvestmentPda::check_address(args.user, &crate::ID, &ctx.investment)?;
    let mut pda = UserInvestmentPda::from_account(&ctx.investment)?;
    let mut amount = args.amount;
    // Investments that won't be touched
    let mut investments: Vec<Investment> = pda
        .investment
        .investments
        .iter()
        .filter(|elt| elt.kind != args.kind)
        .copied()
        .collect();

    // Look for the investements matching the desired type
    // Reduce their amounts until the desired canceled amount is reached
    pda.investment
        .investments
        .iter()
        .filter(|elt| elt.kind == args.kind)
        .for_each(|elt| {
            let mut elt = *elt;
            if elt.amount_bought > amount {
                elt.amount_bought = elt.amount_bought.saturating_sub(amount);
                amount = 0;
                investments.push(elt);
            } else {
                amount = amount.saturating_sub(elt.amount_bought);
            }
        });
    // If the total amount couldn't be removed, then it's an error
    if amount > 0 {
        return Err(Error::InvalidAmount.into());
    }

    // Save the PDA or delete it if there are no investments left
    if investments.is_empty() {
        pda.delete(&ctx.admin1)
    } else {
        pda.investment.investments = investments;
        pda.write(&ctx.admin1)
    }
}

struct LaunchBgkAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> LaunchBgkAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Set the BGK token launch date.
fn launch_bgk(program_id: &Pubkey, accounts: &[AccountInfo], args: LaunchBGKArgs) -> ProgramResult {
    let ctx = LaunchBgkAccounts::new(accounts)?;
    msg!("Bangk: Setting BGK launch date");

    check_pda_owner!(program_id, ctx.config, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);

    ConfigurationPda::check_address(&crate::ID, &ctx.config)?;
    let mut config = ConfigurationPda::from_account(&ctx.config)?;

    if config.launch_date > 0 {
        return Err(Error::BGKTokenAlreadyLaunched.into());
    }

    config.launch_date = args.timestamp;
    config.write(&ctx.admin1)?;

    Ok(())
}

struct VestingReleaseAccounts<'a> {
    api: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    pda_source: AccountInfo<'a>,
    user: AccountInfo<'a>,
    investment: AccountInfo<'a>,
    ata_user: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
    _program_ata: AccountInfo<'a>,
}

impl<'a> VestingReleaseAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            api: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            mint_bgk: next_account_info(accounts_iter)?.clone(),
            pda_source: next_account_info(accounts_iter)?.clone(),
            user: next_account_info(accounts_iter)?.clone(),
            investment: next_account_info(accounts_iter)?.clone(),
            ata_user: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
            _program_ata: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Release vested tokens
fn vesting_release(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let ctx = VestingReleaseAccounts::new(accounts)?;
    msg!("Bangk: releasing vested tokens");

    debug!("Security checks");
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Routine);
    check_pda_owner!(program_id, ctx.config, ctx.investment);

    debug!("Reading PDA data");
    ConfigurationPda::check_address(&crate::ID, &ctx.config)?;
    let config = ConfigurationPda::from_account(&ctx.config)?;
    UserInvestmentPda::check_address(ctx.user.key, &crate::ID, &ctx.investment)?;
    let mut investment = UserInvestmentPda::from_account(&ctx.investment)?;

    debug!("Integrity checks");
    if config.launch_date == 0 {
        return Err(Error::IcoUnvestBeforeLaunch.into());
    }
    check_ata_exists!(ctx.investment);
    if investment.investment.user != *ctx.user.key {
        return Err(Error::AccountOwnerMismatch.into());
    }
    if ctx.ata_user.lamports() > 0 && get_ata_owner(&ctx.ata_user)? != *ctx.user.key {
        return Err(Error::AccountOwnerMismatch.into());
    }
    if *ctx.pda_source.key != WalletType::Ico.get_pda().0 {
        msg!("unexpected address for wallet PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    debug!("Getting Timestamp");
    let now = get_timestamp()?;

    // Get the number of tokens that should be released for the user.
    let mut to_release = 0_u64;
    for invest in &mut investment.investment.investments {
        let rule_released = invest.amount_released;
        let rule = match invest.custom_rule {
            Some(rule) => rule,
            None => *config
                .unvesting
                .get(&invest.kind)
                .ok_or(Error::InvalidUnvestingDefinition)?,
        };
        let rule_available = rule
            .unvested(config.launch_date, now)?
            .saturating_mul(invest.amount_bought)
            .saturating_div(100_000);
        debug!(
            "Rule {:?} has {} tokens available",
            invest.kind, rule_available
        );
        to_release = to_release.saturating_add(rule_available.saturating_sub(rule_released));
        invest.amount_released = rule_available;
    }

    if to_release == 0 {
        return Ok(());
    }
    investment.write(&ctx.api)?;

    // Transferring the required amount of tokens from the invested ATA to the user's ATA
    if ctx.ata_user.lamports() == 0 {
        // Creating the ATA
        debug!("creating the user's ATA");
        invoke(
            &create_associated_token_account(
                ctx.api.key,
                ctx.user.key,
                ctx.mint_bgk.key,
                ctx.program_token.key,
            ),
            &[
                ctx.api.clone(),
                ctx.ata_user.clone(),
                ctx.user.clone(),
                ctx.mint_bgk.clone(),
                ctx.program_system.clone(),
                ctx.program_token.clone(),
            ],
        )?;
    }

    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();

    debug!(
        "transferring {} tokens from the invested ATA to the user's ATA",
        to_release
    );
    invoke_signed(
        &transfer_checked(
            ctx.program_token.key,
            ctx.pda_source.key,
            ctx.mint_bgk.key,
            ctx.ata_user.key,
            ctx.sig_admin.key,
            &[],
            to_release,
            6,
        )?,
        &[
            ctx.pda_source.clone(),
            ctx.mint_bgk.clone(),
            ctx.ata_user.clone(),
            ctx.sig_admin.clone(),
        ],
        &[admin_seeds.as_slice()],
    )
}

struct QueueTransferFromReserveAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    timelock: AccountInfo<'a>,
    _program_system: AccountInfo<'a>,
}

impl<'a> QueueTransferFromReserveAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            timelock: next_account_info(accounts_iter)?.clone(),
            _program_system: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Transfer BGK tokens from Bangk's reserve.
fn queue_transfer_from_reserve(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: QueueTransferFromInternalWalletArgs,
) -> ProgramResult {
    let ctx = QueueTransferFromReserveAccounts::new(accounts)?;
    msg!("Bangk: Queue tranfering BGK tokens from Bangk's reserve");

    check_pda_owner!(program_id, ctx.sig_admin, ctx.timelock);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);

    TimelockPda::check_address(&crate::ID, &ctx.timelock)?;
    let mut timelock_pda = TimelockPda::from_account(&ctx.timelock)?;
    // Create the timelocked instruction
    let timelock = Timelock::transfer_from_internal_wallet(args.source, args.target, args.amount)?;
    timelock_pda.instructions.push(timelock);
    timelock_pda.write(&ctx.admin1)
}

struct ExecuteTransferFromReserveAccounts<'a> {
    admin1: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    timelock: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    pda_source: AccountInfo<'a>,
    user: AccountInfo<'a>,
    ata_target: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
    _program_ata: AccountInfo<'a>,
}

impl<'a> ExecuteTransferFromReserveAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            timelock: next_account_info(accounts_iter)?.clone(),
            mint_bgk: next_account_info(accounts_iter)?.clone(),
            pda_source: next_account_info(accounts_iter)?.clone(),
            user: next_account_info(accounts_iter)?.clone(),
            ata_target: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
            _program_ata: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Transfer BGK tokens from Bangk's reserve.
fn execute_transfer_from_reserve(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ExecuteTransferFromInternalWalletArgs,
) -> ProgramResult {
    let ctx = ExecuteTransferFromReserveAccounts::new(accounts)?;
    msg!("Bangk: Tranfering BGK tokens from Bangk's reserve");

    check_pda_owner!(program_id, ctx.sig_admin, ctx.timelock);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Routine);

    debug!("integrity check on the source wallet");
    if *ctx.pda_source.key != args.source.get_pda().0 {
        msg!("unexpected address for wallet PDA");
        return Err(Error::InvalidPdaAddress.into());
    }

    // Check that there’s a queued transfer, and remove it from the list if found
    TimelockPda::check_address(&crate::ID, &ctx.timelock)?;
    let mut timelock = TimelockPda::from_account(&ctx.timelock)?;
    timelock.process_transfer_from_internal_wallet(
        args.source,
        ctx.ata_target.key,
        args.amount,
        &ctx.admin1,
    )?;
    debug!("queued operation is ready, proceeding");

    // Transferring the required amount of tokens from the reserve ATA to the target ATA
    MultiSigPda::check_address(MultiSigType::Admin, &crate::ID, &ctx.sig_admin)?;
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let admin_seeds = admin_sig.seeds();
    let admin_seeds = admin_seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();

    // Creating the ATA
    if ctx.ata_target.lamports() == 0 {
        debug!("creating the target ATA since it doesn't yet exist");
        invoke(
            &create_associated_token_account(
                ctx.admin1.key,
                ctx.user.key,
                ctx.mint_bgk.key,
                ctx.program_token.key,
            ),
            &[
                ctx.admin1.clone(),
                ctx.ata_target.clone(),
                ctx.user.clone(),
                ctx.mint_bgk.clone(),
                ctx.program_system.clone(),
                ctx.program_token.clone(),
            ],
        )?;
    }

    debug!("transferring the tokens from the reserve ATA to the target ATA");
    invoke_signed(
        &transfer_checked(
            ctx.program_token.key,
            ctx.pda_source.key,
            ctx.mint_bgk.key,
            ctx.ata_target.key,
            ctx.sig_admin.key,
            &[],
            args.amount,
            6,
        )?,
        &[
            ctx.pda_source.clone(),
            ctx.mint_bgk.clone(),
            ctx.ata_target.clone(),
            ctx.sig_admin.clone(),
        ],
        &[admin_seeds.as_slice()],
    )
}
