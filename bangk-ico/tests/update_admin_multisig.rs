// File: tests-onchain-ico/tests/update_admin_multisig.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 24 July 2024 @ 19:01:05
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]

use bangk_ico::update_admin_multisig;
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error,
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer as _;

use crate::common::PROGRAM_ID;

pub mod common;

#[tokio::test]
async fn default() {
    let mut env = common::init_default().await;
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin2 = env.add_wallet("Admin 6").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let Ok(instruction) = update_admin_multisig(
        &admin1,
        &admin2,
        &admin3,
        &new_api_key,
        &new_admin1,
        &new_admin2,
        &new_admin3,
        &new_admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Checking that the keys have been replaced
    let Some(admin): Option<MultiSigPda> = env.from_account(&admin_keys_pda).await else {
        panic!("could not load the admin multisig");
    };
    assert_eq!(admin.multisig.sig_type, MultiSigType::Admin);
    assert_eq!(
        admin.multisig.keys,
        &[new_api_key, new_admin1, new_admin2, new_admin3, new_admin4]
    );
}

#[tokio::test]
async fn old_are_invalid() {
    let mut env = common::init_default().await;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin2 = env.add_wallet("Admin 6").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let Ok(instruction1) = update_admin_multisig(
        &api,
        &admin2,
        &admin4,
        &new_api_key,
        &new_admin1,
        &new_admin2,
        &new_admin3,
        &new_admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res1 = env
        .execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res1.is_ok(),
        "there was an unexpected error in the instruction"
    );

    // Keys are changed, try to change it back with the old signers
    let Ok(instruction2) = update_admin_multisig(
        &api,
        &admin2,
        &admin4,
        &api,
        &new_admin1,
        &admin2,
        &new_admin3,
        &admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res2 = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res2.is_err_and(|err| err == Error::InvalidSigner),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn duplicated_key_in_multisig() {
    let mut env = common::init_default().await;

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let Ok(instruction) = update_admin_multisig(
        &admin1,
        &admin2,
        &admin3,
        &new_api_key,
        &new_admin1,
        &new_admin3,
        &new_admin1,
        &new_admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_err_and(|err| err == Error::DuplicatedKeyInMultisigDefinition),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn use_duplicate_keys() {
    let mut env = common::init_default().await;
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 1"].pubkey();
    let admin3 = env.wallets["Admin 1"].pubkey();

    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin2 = env.add_wallet("Admin 6").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let Ok(instruction) = update_admin_multisig(
        &admin1,
        &admin2,
        &admin3,
        &new_api_key,
        &new_admin1,
        &new_admin2,
        &new_admin3,
        &new_admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 1", "Admin 1"])
        .await;
    assert!(
        res.is_err_and(|err| err == Error::InvalidSigner),
        "the transaction succeeded where it should have failed"
    );
}
