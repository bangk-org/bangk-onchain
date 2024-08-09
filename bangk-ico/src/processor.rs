// File: bangk-ico/src/processor.rs
// Project: bangk-onchain
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 20:47:48
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
    program::{invoke, invoke_signed},
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
    instruction::{initialize_mint2, mint_to, set_authority, transfer_checked, AuthorityType},
    state::Mint,
};
use spl_token_metadata_interface::{
    instruction::initialize as initialize_metadata, state::TokenMetadata,
};

use crate::{
    config::ConfigurationPda,
    instruction::{
        BangkIcoInstruction, CancelInvestmentArgs, InitializeArgs, LaunchBGKArgs, MintCreationArgs,
        TransferFromReserveArgs, UpdateAdminMultisigArgs, UserInvestmentArgs,
    },
    investment::{Investment, UserInvestment, UserInvestmentPda},
    unvesting::UnvestingType,
};

include!(concat!(env!("OUT_DIR"), "/keys.rs"));

const TOTAL_TOKEN_AMOUNT: u64 = 177_000_000_000_000;

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
        BangkIcoInstruction::CancelInvestment(args) => {
            cancel_investment(program_id, accounts, args)
        }
        BangkIcoInstruction::LaunchBGK(args) => launch_bgk(program_id, accounts, args),
        BangkIcoInstruction::VestingRelease => vesting_release(program_id, accounts),
        BangkIcoInstruction::TransferFromReserve(args) => {
            transfer_from_reserve(program_id, accounts, args)
        }
    }
}

struct InitializeAccounts<'a> {
    bangk: AccountInfo<'a>,
    config: AccountInfo<'a>,
    admin_sig: AccountInfo<'a>,
    _system_program: AccountInfo<'a>,
}

impl<'a> InitializeAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            bangk: next_account_info(accounts_iter)?.clone(),
            config: next_account_info(accounts_iter)?.clone(),
            admin_sig: next_account_info(accounts_iter)?.clone(),
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

    if args.unvesting.len() != 6 {
        msg!(
            "Unvesting definition should have 6 elements (got {})",
            args.unvesting.len()
        );
        return Err(Error::InvalidUnvestingDefinition.into());
    }

    if args
        .unvesting
        .iter()
        .map(|def| def.kind)
        .collect::<HashSet<_>>()
        .len()
        != 6
    {
        msg!("unvesting definition had a duplicate and missing type");
        return Err(Error::InvalidUnvestingDefinition.into());
    }

    if args
        .unvesting
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
    let (_config_pda, config_bump) = ConfigurationPda::get_address(&crate::ID);
    let (_admin_keys_pda, admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &crate::ID);

    // Saving the Configuration PDA on the chain.
    debug!("writing config PDA");
    let config = ConfigurationPda::new(config_bump, &args.unvesting, ctx.admin_sig.key);
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

    msg!("ICO program successfully initialized");
    Ok(())
}

struct MintCreationAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    ata_reserve: AccountInfo<'a>,
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
            mint_bgk: next_account_info(accounts_iter)?.clone(),
            ata_reserve: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
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
        uri: "https://bangk.app/bgk_token.json".to_owned(),
        additional_metadata: vec![],
    };

    let meta_len = metadata
        .tlv_size_of()
        .map_err(|_err| Error::InvalidRawData)?;

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
        ctx.admin1.key,
        ctx.mint_bgk.key,
        rent,
        mint_len as u64,
        &spl_token_2022::id(),
    );

    invoke_signed(
        &create_pda_instr,
        &[ctx.admin1.clone(), ctx.mint_bgk.clone()],
        &[&[b"Mint", b"BGK", &[args.bump]]],
    )?;

    debug!("Initializing extensions");
    let seeds = admin_sig.seeds();
    let seeds = seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
    invoke_signed(
        &metadata_pointer::instruction::initialize(
            &spl_token_2022::id(),
            ctx.mint_bgk.key,
            Some(*ctx.sig_admin.key),
            Some(*ctx.mint_bgk.key),
        )?,
        &[ctx.mint_bgk.clone(), ctx.sig_admin.clone()],
        &[seeds.as_slice()],
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
        &[seeds.as_slice()],
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
        &[seeds.as_slice()],
    )?;

    debug!("Mint successfully initialized");
    invoke_signed(
        &create_associated_token_account(
            ctx.admin1.key,
            ctx.sig_admin.key,
            ctx.mint_bgk.key,
            ctx.program_token.key,
        ),
        &[
            ctx.admin1.clone(),
            ctx.ata_reserve.clone(),
            ctx.sig_admin.clone(),
            ctx.mint_bgk.clone(),
            ctx.program_system.clone(),
            ctx.program_token.clone(),
        ],
        &[seeds.as_slice()],
    )?;

    debug!("integrity check on the target ATA");
    let target_ata = get_associated_token_address_with_program_id(
        ctx.sig_admin.key,
        ctx.mint_bgk.key,
        &spl_token_2022::ID,
    );

    if target_ata != *ctx.ata_reserve.key {
        msg!(
            "the given target ATA was not the expected one ({})",
            target_ata
        );
        return Err(Error::InvalidAta.into());
    }

    debug!("Minting tokens");
    invoke_signed(
        &mint_to(
            &spl_token_2022::id(),
            ctx.mint_bgk.key,
            ctx.ata_reserve.key,
            ctx.sig_admin.key,
            &[ctx.sig_admin.key],
            TOTAL_TOKEN_AMOUNT,
        )?,
        &[
            ctx.mint_bgk.clone(),
            ctx.ata_reserve.clone(),
            ctx.sig_admin.clone(),
        ],
        &[seeds.as_slice()],
    )?;

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
        &[seeds.as_slice()],
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

    let mut admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    admin_sig.multisig.keys = vec![
        args.api_key,
        args.admin1,
        args.admin2,
        args.admin3,
        args.admin4,
    ];
    admin_sig.write(&ctx.sig_admin, &ctx.admin1)
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

    let mut config = ConfigurationPda::from_account(&ctx.config)?;
    config.amount_invested = config.amount_invested.saturating_add(args.amount);
    config.write(&ctx.config, &ctx.api)?;

    if args.invest_kind != UnvestingType::AdvisersPartners
        && config.launch_date > 0
        && config.launch_date <= get_timestamp()?
    {
        return Err(Error::IcoInvestAfterLaunch.into());
    }

    // If PdA doesn't exist yet, create it, otherwise update it
    if ctx.investment.lamports() == 0 {
        let investment =
            UserInvestment::new(args.user, args.invest_kind, args.amount, args.custom_rule)?;
        let pda = UserInvestmentPda::new(args.bump, investment);
        pda.create(&ctx.investment, &ctx.api, &crate::ID)
    } else {
        let mut pda = UserInvestmentPda::from_account(&ctx.investment)?;
        pda.investment.investments.push(Investment {
            kind: args.invest_kind,
            timestamp: get_timestamp()?,
            custom_rule: args.custom_rule,
            amount_bought: args.amount,
            amount_released: 0,
        });
        pda.write(&ctx.investment, &ctx.api)
    }
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

    let mut config = ConfigurationPda::from_account(&ctx.config)?;
    config.amount_invested = config.amount_invested.saturating_sub(args.amount);
    config.write(&ctx.config, &ctx.admin1)?;

    if config.launch_date > 0 {
        return Err(Error::CancelIcoInvestmentAfterLaunch.into());
    }

    // If PdA doesn't exist that's an error
    if ctx.investment.lamports() == 0 {
        return Err(Error::InvestmentDoesNotExist.into());
    }

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
        pda.delete(&ctx.investment, &ctx.admin1)
    } else {
        pda.investment.investments = investments;
        pda.write(&ctx.investment, &ctx.admin1)
    }
}

struct LaunchBgkAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    ata_reserve: AccountInfo<'a>,
    ata_invested: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
    _program_ata: AccountInfo<'a>,
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
            mint_bgk: next_account_info(accounts_iter)?.clone(),
            ata_reserve: next_account_info(accounts_iter)?.clone(),
            ata_invested: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
            _program_ata: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Set the BGK token launch date.
fn launch_bgk(program_id: &Pubkey, accounts: &[AccountInfo], args: LaunchBGKArgs) -> ProgramResult {
    let ctx = LaunchBgkAccounts::new(accounts)?;
    msg!("Bangk: Setting BGK launch date");

    check_pda_owner!(program_id, ctx.config, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);
    check_ata_exists!(ctx.ata_reserve);

    let mut config = ConfigurationPda::from_account(&ctx.config)?;

    if config.launch_date > 0 {
        return Err(Error::BGKTokenAlreadyLaunched.into());
    }
    if ctx.ata_invested.lamports() > 0 {
        msg!("Invested ATA already exists: aborting to prevent any risk.");
        return Err(Error::AccountAlreadyExists.into());
    }

    if config.amount_invested != args.amount {
        msg!("The number of tokens to transfer to invested ATA does not match the amount invested ({})", config.amount_invested);
        return Err(Error::InvalidInvestedAmount.into());
    }

    config.launch_date = args.timestamp;
    config.write(&ctx.config, &ctx.admin1)?;

    // Transferring the required amount of tokens from the reserve ATA to the invested ATA
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let seeds = admin_sig.seeds();
    let seeds = seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
    // Creating the ATA
    debug!("creating the invested ATA");
    invoke_signed(
        &create_associated_token_account(
            ctx.admin1.key,
            ctx.config.key,
            ctx.mint_bgk.key,
            ctx.program_token.key,
        ),
        &[
            ctx.admin1.clone(),
            ctx.ata_invested.clone(),
            ctx.config.clone(),
            ctx.mint_bgk.clone(),
            ctx.program_system.clone(),
            ctx.program_token.clone(),
        ],
        &[seeds.as_slice()],
    )?;

    debug!("transferring the tokens from the reserve ATA to the invested ATA");
    invoke_signed(
        &transfer_checked(
            ctx.program_token.key,
            ctx.ata_reserve.key,
            ctx.mint_bgk.key,
            ctx.ata_invested.key,
            ctx.sig_admin.key,
            &[],
            args.amount,
            6,
        )?,
        &[
            ctx.ata_reserve.clone(),
            ctx.mint_bgk.clone(),
            ctx.ata_invested.clone(),
            ctx.sig_admin.clone(),
        ],
        &[seeds.as_slice()],
    )
}

struct VestingReleaseAccounts<'a> {
    api: AccountInfo<'a>,
    config: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    ata_source: AccountInfo<'a>,
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
            ata_source: next_account_info(accounts_iter)?.clone(),
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
    let config = ConfigurationPda::from_account(&ctx.config)?;
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
    investment.write(&ctx.investment, &ctx.api)?;

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
    let seeds = config.seeds();
    let seeds = seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();

    debug!(
        "transferring {} tokens from the invested ATA to the user's ATA",
        to_release
    );
    invoke_signed(
        &transfer_checked(
            ctx.program_token.key,
            ctx.ata_source.key,
            ctx.mint_bgk.key,
            ctx.ata_user.key,
            ctx.config.key,
            &[],
            to_release,
            6,
        )?,
        &[
            ctx.ata_source.clone(),
            ctx.mint_bgk.clone(),
            ctx.ata_user.clone(),
            ctx.config.clone(),
        ],
        &[seeds.as_slice()],
    )
}

struct TransferFromReserveAccounts<'a> {
    admin1: AccountInfo<'a>,
    _admin2: AccountInfo<'a>,
    _admin3: AccountInfo<'a>,
    sig_admin: AccountInfo<'a>,
    mint_bgk: AccountInfo<'a>,
    ata_reserve: AccountInfo<'a>,
    user: AccountInfo<'a>,
    ata_target: AccountInfo<'a>,
    program_system: AccountInfo<'a>,
    program_token: AccountInfo<'a>,
    _program_ata: AccountInfo<'a>,
}

impl<'a> TransferFromReserveAccounts<'a> {
    fn new(accounts: &[AccountInfo<'a>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(Self {
            admin1: next_account_info(accounts_iter)?.clone(),
            _admin2: next_account_info(accounts_iter)?.clone(),
            _admin3: next_account_info(accounts_iter)?.clone(),
            sig_admin: next_account_info(accounts_iter)?.clone(),
            mint_bgk: next_account_info(accounts_iter)?.clone(),
            ata_reserve: next_account_info(accounts_iter)?.clone(),
            user: next_account_info(accounts_iter)?.clone(),
            ata_target: next_account_info(accounts_iter)?.clone(),
            program_system: next_account_info(accounts_iter)?.clone(),
            program_token: next_account_info(accounts_iter)?.clone(),
            _program_ata: next_account_info(accounts_iter)?.clone(),
        })
    }
}

/// Transfer BGK tokens from Bangk's reserve.
fn transfer_from_reserve(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: TransferFromReserveArgs,
) -> ProgramResult {
    let ctx = TransferFromReserveAccounts::new(accounts)?;
    msg!("Bangk: Tranfering BGK tokens from Bangk's reserve");

    check_pda_owner!(program_id, ctx.sig_admin);
    check_signers!(accounts, &ctx.sig_admin, OperationSecurityLevel::Critical);
    check_ata_exists!(ctx.ata_reserve);

    debug!("integrity check on the target ATA");
    let target_ata = get_associated_token_address_with_program_id(
        ctx.sig_admin.key,
        ctx.mint_bgk.key,
        &spl_token_2022::ID,
    );

    if target_ata != *ctx.ata_reserve.key {
        msg!(
            "the given target ATA was not the expected one ({})",
            target_ata
        );
        return Err(Error::InvalidAta.into());
    }

    // Transferring the required amount of tokens from the reserve ATA to the target ATA
    let admin_sig = MultiSigPda::from_account(&ctx.sig_admin)?;
    let seeds = admin_sig.seeds();
    let seeds = seeds.iter().map(Vec::as_slice).collect::<Vec<_>>();
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
            ctx.ata_reserve.key,
            ctx.mint_bgk.key,
            ctx.ata_target.key,
            ctx.sig_admin.key,
            &[],
            args.amount,
            6,
        )?,
        &[
            ctx.ata_reserve.clone(),
            ctx.mint_bgk.clone(),
            ctx.ata_target.clone(),
            ctx.sig_admin.clone(),
        ],
        &[seeds.as_slice()],
    )
}
