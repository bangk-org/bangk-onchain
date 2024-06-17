// File: tests-onchain-ico/tests/launch.rs
// Project: bangk-solana
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 17 June 2024 @ 09:46:14
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::too_many_lines)]
#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::indexing_slicing)]

pub mod common;
use bangk_ico::{config::IcoConfigurationPda, instruction::launch_bgk};
use bangk_onchain_common::{
    errors::BangkError,
    security::{MultiSigPda, MultiSigType},
};
use common::{PROGRAM_ID, TOTAL_BGK_TOKENS};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};

const TIMESTAMP: i64 = 19829;
const AMOUNT: u64 = 57_000_000_000_000;

#[tokio::test]
async fn default() {
    let mut env = common::init_with_mint().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();

    let (config_pda, _config_bump) = IcoConfigurationPda::get_address();
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );
    let invested_ata = get_associated_token_address_with_program_id(
        &config_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Create the investment
    let Ok(instruction1) = launch_bgk(&api, &admin2, &admin4, TIMESTAMP, AMOUNT) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    assert!(env
        .from_account::<IcoConfigurationPda>(&config_pda)
        .await
        .is_some_and(|config| config.launch_date == TIMESTAMP));
    assert_eq!(
        env.get_token_amount(&reserve_ata).await,
        Some(TOTAL_BGK_TOKENS - AMOUNT)
    );
    assert_eq!(env.get_token_amount(&invested_ata).await, Some(AMOUNT));
}

#[tokio::test]
async fn double_launch() {
    let mut env = common::init_with_mint().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();

    // Create the investment
    let Ok(instruction1) = launch_bgk(&api, &admin2, &admin4, TIMESTAMP, AMOUNT) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    let Ok(instruction2) = launch_bgk(&api, &admin2, &admin4, TIMESTAMP + 2, AMOUNT) else {
        panic!("could not create instruction");
    };
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res2.is_err_and(|err| err == BangkError::BGKTokenAlreadyLaunched),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn ata_already_exists() {
    let mut env = common::init_with_mint().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let (config_pda, _config_bump) = IcoConfigurationPda::get_address();
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);

    let instruction1 =
        create_associated_token_account(&api, &config_pda, &mint_address, &spl_token_2022::ID);
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Launch the token
    let Ok(instruction2) = launch_bgk(&api, &admin2, &admin4, TIMESTAMP, AMOUNT) else {
        panic!("could not create instruction");
    };
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res2.is_err_and(|err| err == BangkError::AccountAlreadyExists),
        "there was an unexpected error in the instruction"
    );
}
