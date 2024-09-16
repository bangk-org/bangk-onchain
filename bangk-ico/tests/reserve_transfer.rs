// File: bangk-ico/tests/reserve_transfer.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 12:23:28
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic_in_result_fn)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

pub mod common;
use std::thread::sleep;
use std::time::Duration;
use std::{error, result};

use bangk_ico::{
    execute_transfer_from_internal_wallet, queue_transfer_from_internal_wallet, WalletType,
    TIMELOCK_DELAY,
};
use bangk_onchain_common::Error as BangkError;
use common::{PROGRAM_ID, TOTAL_RESERVE_TOKENS};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};

const AMOUNT: u64 = 10_000_000;

#[tokio::test]
async fn to_non_existing_ata() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_pda = WalletType::Reserve.get_pda().0;
    let target_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);

    // Queue the transfer
    let instruction1 = queue_transfer_from_internal_wallet(
        &api,
        &admin1,
        &admin4,
        &user,
        WalletType::Reserve,
        AMOUNT,
    )?;
    env.execute_transaction(&[instruction1], &["API", "Admin 1", "Admin 4"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction2 =
        execute_transfer_from_internal_wallet(&api, &user, WalletType::Reserve, AMOUNT)?;
    env.execute_transaction(&[instruction2], &["API"]).await?;

    assert_eq!(
        env.get_token_amount(&reserve_pda).await,
        Some(TOTAL_RESERVE_TOKENS - AMOUNT)
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT));

    Ok(())
}

#[tokio::test]
async fn to_already_existing_ata() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_pda = WalletType::Reserve.get_pda().0;
    let target_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);

    // Creating the ATA first
    let instruction1 =
        create_associated_token_account(&api, &user, &mint_address, &spl_token_2022::ID);
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Transfer the tokens
    let instruction2 = queue_transfer_from_internal_wallet(
        &api,
        &admin1,
        &admin4,
        &user,
        WalletType::Reserve,
        AMOUNT,
    )?;
    env.execute_transaction(&[instruction2], &["API", "Admin 4", "Admin 1"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction3 =
        execute_transfer_from_internal_wallet(&api, &user, WalletType::Reserve, AMOUNT)?;
    env.execute_transaction(&[instruction3], &["API"]).await?;

    assert_eq!(
        env.get_token_amount(&reserve_pda).await,
        Some(TOTAL_RESERVE_TOKENS - AMOUNT),
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT),);

    Ok(())
}

#[tokio::test]
async fn not_waiting_for_delay() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);

    // Creating the ATA first
    let instruction1 =
        create_associated_token_account(&api, &user, &mint_address, &spl_token_2022::ID);
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Transfer the tokens
    let instruction2 = queue_transfer_from_internal_wallet(
        &api,
        &admin1,
        &admin4,
        &user,
        WalletType::Reserve,
        AMOUNT,
    )?;
    env.execute_transaction(&[instruction2], &["API", "Admin 1", "Admin 4"])
        .await?;
    // Execute the instruction
    let instruction3 =
        execute_transfer_from_internal_wallet(&api, &user, WalletType::Reserve, AMOUNT)?;
    let res = env.execute_transaction(&[instruction3], &["API"]).await;
    assert!(
        res.as_ref()
            .is_err_and(|err| *err == BangkError::QueuedInstructionNotReady),
        "{res:#?}"
    );

    Ok(())
}

#[tokio::test]
async fn from_foundation() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_pda = WalletType::Foundation.get_pda().0;
    let target_ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::ID);

    // Creating the ATA first
    let instruction1 =
        create_associated_token_account(&api, &user, &mint_address, &spl_token_2022::ID);
    env.execute_transaction(&[instruction1], &["API"]).await?;

    // Transfer the tokens
    let instruction2 = queue_transfer_from_internal_wallet(
        &api,
        &admin1,
        &admin4,
        &user,
        WalletType::Foundation,
        AMOUNT,
    )?;
    env.execute_transaction(&[instruction2], &["API", "Admin 1", "Admin 4"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction3 =
        execute_transfer_from_internal_wallet(&api, &user, WalletType::Foundation, AMOUNT)?;
    env.execute_transaction(&[instruction3], &["API"]).await?;

    assert_eq!(
        env.get_token_amount(&reserve_pda).await,
        Some(14_000_000_000_000 - AMOUNT),
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT),);

    Ok(())
}

#[tokio::test]
async fn missing_president_signature() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let user = Pubkey::new_unique();

    // Transfer the tokens
    let instruction2 = queue_transfer_from_internal_wallet(
        &api,
        &admin2,
        &admin4,
        &user,
        WalletType::Foundation,
        AMOUNT,
    )?;
    let res = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;

    assert!(res.is_err_and(|err| err == BangkError::MissingPresidentSignature));

    Ok(())
}
