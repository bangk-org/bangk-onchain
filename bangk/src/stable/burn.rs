// File: bangk/src/stable/burn.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/stable/burn.rs
// Project: bangk-onchain
// Creation date: Wednesday 06 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 27 June 2024 @ 11:24:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use crate::{instruction::BurnStableCoinsArgs, state::stable::StableMint, utils::tokens::burn};
use bangk_onchain_common::{check_ata_exists, debug};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
};

/// Remove stable coins from a client's account.
///
/// # Parameters
/// * `accounts` - Accounts used in the transaction,
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 󰴒 Transaction payer,
/// 2. 󰴒 Mint the tokens to burn belong to,
/// 3. 󰴒 Associated Token Account owning the tokens to burn,
/// 4. SPL 2022 Token program.
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(accounts: &[AccountInfo], args: BurnStableCoinsArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let ata = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;

    msg!("Bangk: burning stable tokens");

    // check_signers!(accounts, payer);
    check_ata_exists!(ata);

    let _stable_mint: StableMint = mint.clone().try_into()?;
    debug!("Burning {} from account {}", _stable_mint.name, ata.key);
    burn(
        ata,
        mint,
        payer,
        program_spl2022,
        args.amount,
        args.close_empty,
    )
}
