// File: bangk/src/stable/transfer.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/stable/transfer.rs
// Project: bangk-onchain
// Creation date: Wednesday 06 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 27 June 2024 @ 11:24:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use crate::{
    instruction::TokenAmountArgs,
    state::stable::StableMint,
    utils::tokens::{transfer, FreezeCheck},
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
/// 2. Mint the tokens to transfer are associated to,
/// 3. 󰴒 Source ATA,
/// 4. 󰴒 Target ATA (will be created if needed),
/// 5. SPL 2022 Token program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: TokenAmountArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let ata_from = next_account_info(accounts_iter)?;
    let ata_to = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: transferring stable tokens");

    // Integrity checks
    // check_signers!(accounts, payer);
    // check_bangk_owner!(program_id, mint);
    check_ata_exists!(ata_from, ata_to);
    check_spl_program!(program_spl2022);

    let _stable_mint: StableMint = mint.clone().try_into()?;
    debug!(
        "Transferring {} from account {} to account {}",
        _stable_mint.name, ata_from.key, ata_to.key
    );

    transfer(
        ata_from,
        ata_to,
        mint,
        payer,
        program_spl2022,
        FreezeCheck::Neither,
        args.amount,
    )?;

    Ok(())
}
