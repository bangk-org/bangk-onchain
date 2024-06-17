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

pub mod common;
use bangk_ico::transfer_from_reserve;
use bangk_onchain_common::security::{MultiSigPda, MultiSigType};
use common::{PROGRAM_ID, TOTAL_BGK_TOKENS};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};

const AMOUNT: u64 = 10_000_000;

#[tokio::test]
async fn to_non_existing_ata() {
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

    // Transfer the tokens
    let Ok(instruction1) = transfer_from_reserve(&api, &admin2, &admin4, &user, AMOUNT) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );
    assert_eq!(
        env.get_token_amount(&reserve_ata).await,
        Some(TOTAL_BGK_TOKENS - AMOUNT)
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT));
}

#[tokio::test]
async fn to_already_existing_ata() {
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
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Transfer the tokens
    let Ok(instruction2) = transfer_from_reserve(&api, &admin2, &admin4, &user, AMOUNT) else {
        panic!("could not create instruction");
    };
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res2.is_ok(),
        "there was an unexpected error in the instruction"
    );

    assert_eq!(
        env.get_token_amount(&reserve_ata).await,
        Some(TOTAL_BGK_TOKENS - AMOUNT),
    );
    assert_eq!(env.get_token_amount(&target_ata).await, Some(AMOUNT),);
}
