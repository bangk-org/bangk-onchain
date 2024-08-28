// File: bangk-ico/tests/update_admin_multisig.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 21 August 2024 @ 19:33:07
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

use bangk_ico::update_admin_multisig;
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer as _;

use crate::common::PROGRAM_ID;

pub mod common;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_default().await?;
    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin2 = env.add_wallet("Admin 6").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let instruction = update_admin_multisig(
        &admin1,
        &admin2,
        &admin3,
        &new_api_key,
        &new_admin1,
        &new_admin2,
        &new_admin3,
        &new_admin4,
    )?;
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    // Checking that the keys have been replaced
    let admin: MultiSigPda = env
        .from_account(&admin_keys_pda)
        .await
        .ok_or("could not load the admin multisig")?;
    assert_eq!(admin.multisig.sig_type, MultiSigType::Admin);
    assert_eq!(
        admin.multisig.keys,
        &[new_api_key, new_admin1, new_admin2, new_admin3, new_admin4]
    );

    Ok(())
}

#[tokio::test]
async fn old_are_invalid() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin4 = env.wallets["Admin 4"].pubkey();
    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin2 = env.add_wallet("Admin 6").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let instruction1 = update_admin_multisig(
        &api,
        &admin2,
        &admin4,
        &new_api_key,
        &new_admin1,
        &new_admin2,
        &new_admin3,
        &new_admin4,
    )?;
    env.execute_transaction(&[instruction1], &["API", "Admin 2", "Admin 4"])
        .await?;

    // Keys are changed, try to change it back with the old signers
    let instruction2 = update_admin_multisig(
        &api,
        &admin2,
        &admin4,
        &api,
        &new_admin1,
        &admin2,
        &new_admin3,
        &admin4,
    )?;
    let res = env
        .execute_transaction(&[instruction2], &["API", "Admin 2", "Admin 4"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::InvalidSigner),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn duplicated_key_in_multisig() -> Result<()> {
    let mut env = common::init_default().await?;

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let instruction = update_admin_multisig(
        &admin1,
        &admin2,
        &admin3,
        &new_api_key,
        &new_admin1,
        &new_admin3,
        &new_admin1,
        &new_admin4,
    )?;
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::DuplicatedKeyInMultisigDefinition),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn use_duplicate_keys() -> Result<()> {
    let mut env = common::init_default().await?;
    let (_admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 1"].pubkey();
    let admin3 = env.wallets["Admin 1"].pubkey();

    let new_api_key = env.add_wallet("API 2").await;
    let new_admin1 = env.add_wallet("Admin 5").await;
    let new_admin2 = env.add_wallet("Admin 6").await;
    let new_admin3 = env.add_wallet("Admin 7").await;
    let new_admin4 = env.add_wallet("Admin 8").await;
    let instruction = update_admin_multisig(
        &admin1,
        &admin2,
        &admin3,
        &new_api_key,
        &new_admin1,
        &new_admin2,
        &new_admin3,
        &new_admin4,
    )?;
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 1", "Admin 1"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::InvalidSigner),
        "the transaction succeeded where it should have failed"
    );

    Ok(())
}
