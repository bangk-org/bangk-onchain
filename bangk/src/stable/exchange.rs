// File: bangk/src/stable/exchange.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/stable/exchange.rs
// Project: bangk-onchain
// Creation date: Wednesday 06 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 27 June 2024 @ 11:24:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use crate::{
    instruction::ExchangeStableCoinsArgs,
    state::stable::StableMint,
    utils::tokens::{transfer_with_exchange, ExchangeTrueValue},
};
use bangk_onchain_common::{check_ata_exists, check_spl_program, debug};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

/// Transfers stable coins from one account to another.
///
/// If the destination account does not exist yet, the Associated Token Program is created,
/// before the coins are transferred.
///
/// # Parameters
/// * `accounts` - Accounts used in the transaction.
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. Stable mint of the source currency,
/// 3. 󰴒 Source ATA,
/// 4. 󰴒 Bangk ATA for the source currency,
/// 5. Stable mint of the target currency,
/// 6. 󰴒 Target ATA,
/// 7. 󰴒 Bangk ATA for the target currency,
/// 8. SPL 2022 Token program account.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: ExchangeStableCoinsArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_source = next_account_info(accounts_iter)?;
    let ata_source = next_account_info(accounts_iter)?;
    let exchange_source = next_account_info(accounts_iter)?;
    let mint_target = next_account_info(accounts_iter)?;
    let ata_target = next_account_info(accounts_iter)?;
    let exchange_target = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: exchange stable tokens");

    // Integrity checks
    // check_signers!(accounts, payer);
    // check_bangk_owner!(
    //     program_id,
    //     mint_source,
    //     mint_target,
    //     exchange_source,
    //     exchange_target
    // );
    check_ata_exists!(ata_source, ata_target);
    check_spl_program!(program_spl2022);

    let _stable_mint_source: StableMint = mint_source.clone().try_into()?;
    let _stable_mint_target: StableMint = mint_target.clone().try_into()?;
    debug!(
        "Exchanging {} to {} from account {} to {}",
        _stable_mint_source.name, _stable_mint_target.name, ata_source.key, ata_target.key
    );

    transfer_with_exchange(
        ata_source,
        ata_target,
        mint_source,
        mint_target,
        exchange_source,
        exchange_target,
        payer,
        args.amount,
        args.exchange_rate,
        ExchangeTrueValue::Target,
    )?;
    Ok(())
}
