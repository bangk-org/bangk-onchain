// File: bangk/src/stable/mint.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: bangk/src/stable/mint.rs
// Project: bangk-onchain
// Creation date: Wednesday 06 December 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 11 July 2024 @ 13:46:37
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use crate::{
    instruction::TokenAmountArgs,
    state::{mints::BangkMint, stable::StableMint},
    utils::tokens::mint,
};
use bangk_onchain_common::{
    check_ata_exists, check_ata_program, check_spl_program, check_system_program, debug, Error,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
};

/// Adds stable coins to a client's account.
///
/// The stable account is assumed to have already been created.
///
/// # Parameters
/// * `accounts` - Accounts used in the transaction,
/// * `args` - Arguments to the instruction.
///
/// # Accounts
/// 1. 󰴹 Transaction payer,
/// 2. 󰴒 Stable mint from which to mint tokens,
/// 3. 󰴒 ATA where the tokens will be minted (will be created if needed),
/// 4. System program,
/// 5. SPL 2022 Token program,
///
/// # Errors
/// If the wrong number of accounts was given, if the funds are insufficient, etc.
pub fn process(accounts: &[AccountInfo], args: TokenAmountArgs) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let mint_stable = next_account_info(accounts_iter)?;
    let ata = next_account_info(accounts_iter)?;
    let program_system = next_account_info(accounts_iter)?;
    let program_spl2022 = next_account_info(accounts_iter)?;
    let program_ata = next_account_info(accounts_iter)?;

    msg!("Bangk: minting stable tokens");

    // Integrity checks
    // check_signers!(accounts, payer);
    // check_bangk_owner!(mint_stable);
    check_ata_exists!(ata);
    check_system_program!(program_system);
    check_spl_program!(program_spl2022);
    check_ata_program!(program_ata);

    let stable_mint: BangkMint<StableMint> = mint_stable.clone().try_into()?;
    debug!("Minting {} to account {}", stable_mint.data.name, ata.key);

    if args.amount == 0 {
        return Err(Error::InvalidAmount.into());
    }

    mint(ata, mint_stable, payer, program_spl2022, args.amount)?;

    let _state = stable_mint.state()?;
    debug!(
        "Mint {} now has a supply of {}",
        stable_mint.data.name, _state.supply
    );
    Ok(())
}
