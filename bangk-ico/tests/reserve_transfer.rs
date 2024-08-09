// File: tests-onchain-ico/tests/reserve_transfer.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 24 July 2024 @ 19:00:57
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

pub mod common;
use std::thread::sleep;
use std::time::Duration;
use std::{error, result};

use bangk_ico::{execute_transfer_from_reserve, queue_transfer_from_reserve, TIMELOCK_DELAY};
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use common::{PROGRAM_ID, TOTAL_BGK_TOKENS};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};

const AMOUNT: u64 = 10_000_000;

#[tokio::test]
async fn to_non_existing_ata() -> Result<()> {
    let mut env = common::init_with_mint().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );
    let target_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);

    // Queue the transfer
    let instruction1 = queue_transfer_from_reserve(&api, &admin2, &admin4, &user, AMOUNT)?;
    env.execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction2 = execute_transfer_from_reserve(&api, &user, AMOUNT)?;
    env.execute_transaction(&[instruction2], &["API"]).await?;

    assert_eq!(
        env.get_token_amount(&reserve_ata).await,
        Some(TOTAL_BGK_TOKENS - AMOUNT)
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT));

    Ok(())
}

#[tokio::test]
async fn to_already_existing_ata() -> Result<()> {
    let mut env = common::init_with_mint().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );
    let target_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);

    // Creating the ATA first
    let instruction1 =
        create_associated_token_account(&api, &user, &mint_address, &spl_token_2022::ID);
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Transfer the tokens
    let instruction2 = queue_transfer_from_reserve(&api, &admin2, &admin4, &user, AMOUNT)?;
    env.execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction3 = execute_transfer_from_reserve(&api, &user, AMOUNT)?;
    env.execute_transaction(&[instruction3], &["API"]).await?;

    assert_eq!(
        env.get_token_amount(&reserve_ata).await,
        Some(TOTAL_BGK_TOKENS - AMOUNT),
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT),);

    Ok(())
}

#[tokio::test]
async fn not_waiting_for_delay() -> Result<()> {
    let mut env = common::init_with_mint().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);

    // Creating the ATA first
    let instruction1 =
        create_associated_token_account(&api, &user, &mint_address, &spl_token_2022::ID);
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Transfer the tokens
    let instruction2 = queue_transfer_from_reserve(&api, &admin2, &admin4, &user, AMOUNT)?;
    env.execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await?;
    // Execute the instruction
    let instruction3 = execute_transfer_from_reserve(&api, &user, AMOUNT)?;
    let res = env.execute_transaction(&[instruction3], &["API"]).await;
    assert!(
        res.as_ref()
            .is_err_and(|err| *err == BangkError::QueuedInstructionNotReady),
        "{res:#?}"
    );

    Ok(())
}
