// File: bangk-stable/tests/admin.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 31 December 2024 @ 16:43:47
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic_in_result_fn)]
#![allow(clippy::print_stdout)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

pub mod common;

use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use bangk_stable::{
    add_freeze_authority, initialize, process_instruction, update_admin_multisig, ConfigurationPda,
};
use common::{
    add_freeze_key, close_account, freeze_ata, init_default, mint_coins, remove_freeze_key,
    thaw_ata, transfer_coins,
};
use solana_program_test::{processor, tokio};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use tests_utilities::onchain::Environment;

use crate::common::PROGRAM_ID;

const FREEZE_USER1: &str = "Freeze 1";
const FREEZE_USER2: &str = "Freeze 2";

const CURRENCY: &str = "Euro BANGK";
const SYMBOL: &str = "EUB";
const URI: &str = "https://api.bangk.app/token-eub";
const DECIMALS: u8 = 2;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env =
        Environment::new(PROGRAM_ID, "bangk_stable", processor!(process_instruction)).await;
    let api_key = env
        .wallets
        .get("API")
        .ok_or("no API key in the environment")?;
    let api_pub = api_key.pubkey();

    let admin1 = Pubkey::new_unique();
    let admin2 = Pubkey::new_unique();
    let admin3 = Pubkey::new_unique();
    let admin4 = Pubkey::new_unique();

    let instruction = initialize(
        &api_key.pubkey(),
        &api_key.pubkey(),
        &admin1,
        &admin2,
        &admin3,
        &admin4,
    );
    env.execute_transaction(&[instruction], &["API"]).await?;

    let (config_pda, _) = ConfigurationPda::get_address(&bangk_stable::ID);
    let (admin_pda, _) = MultiSigPda::get_address(MultiSigType::Admin, &env.program_id);

    // Testing configuration PDA integrity
    let config: ConfigurationPda = env
        .from_account(&config_pda)
        .await
        .ok_or("could not load the Stable program configuration")?;
    assert_eq!(
        config.admin_multisig, admin_pda,
        "error in the address for the admin PDA"
    );

    // Testing Admin Keys PDA
    let admin: MultiSigPda = env
        .from_account(&admin_pda)
        .await
        .ok_or("could not load the admin multisig")?;
    assert_eq!(admin.multisig.sig_type, MultiSigType::Admin);
    assert_eq!(
        admin.multisig.keys,
        &[api_pub, admin1, admin2, admin3, admin4]
    );

    Ok(())
}

#[tokio::test]
async fn update_multisig() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let api2 = env.add_wallet("API 2").await;
    let admin5 = env.add_wallet("Admin 5").await;
    let admin6 = env.add_wallet("Admin 6").await;
    let admin7 = env.add_wallet("Admin 7").await;
    let admin8 = env.add_wallet("Admin 8").await;
    let instruction1 = update_admin_multisig(
        &api, &admin1, &admin2, &api2, &admin5, &admin6, &admin7, &admin8,
    );

    env.execute_transaction(&[instruction1], &["API", "Admin 1", "Admin 2"])
        .await?;

    // Operation with initial/default signers will fail
    let res1 = add_freeze_key(&mut env, FREEZE_USER1).await;
    assert_eq!(res1, Err(BangkError::InvalidSigner));

    // Operation with new signers will succeed
    let user = env.add_wallet("New Freeze Auth").await;
    let instruction2 = add_freeze_authority(&api2, &admin7, &admin5, &user);
    env.execute_transaction(&[instruction2], &["API 2", "Admin 7", "Admin 5"])
        .await?;

    Ok(())
}

#[tokio::test]
async fn update_multisig_duplicate() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let api2 = env.add_wallet("API 2").await;
    let admin5 = env.add_wallet("Admin 5").await;
    let admin6 = env.add_wallet("Admin 6").await;
    let admin7 = env.add_wallet("Admin 7").await;
    let instruction1 = update_admin_multisig(
        &api, &admin1, &admin2, &api2, &admin5, &admin6, &admin7, &admin7,
    );

    let res = env
        .execute_transaction(&[instruction1], &["API", "Admin 1", "Admin 2"])
        .await;
    assert_eq!(res, Err(BangkError::DuplicatedKeyInMultisigDefinition));

    Ok(())
}

#[tokio::test]
async fn wrong_signer() -> Result<()> {
    let mut env =
        Environment::new(PROGRAM_ID, "bangk_stable", processor!(process_instruction)).await;

    let random = env.add_wallet("random").await;

    println!("Initializing program");
    let instruction = initialize(
        &random,
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
    );
    let res = env.execute_transaction(&[instruction], &["random"]).await;
    println!("{res:?}");
    assert_eq!(res, Err(BangkError::InvalidSigner));

    Ok(())
}

#[tokio::test]
async fn double_init() -> Result<()> {
    let mut env = init_default().await?;
    let api_key = env
        .wallets
        .get("API")
        .ok_or("no API key in the environment")?;
    let admin1 = Pubkey::new_unique();
    let admin2 = Pubkey::new_unique();
    let admin3 = Pubkey::new_unique();
    let admin4 = Pubkey::new_unique();

    let instruction = initialize(
        &api_key.pubkey(),
        &api_key.pubkey(),
        &admin1,
        &admin2,
        &admin3,
        &admin4,
    );
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert_eq!(
        res,
        Err(BangkError::UniqueOperationAlreadyExecuted),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn duplicated_key_in_multisig() -> Result<()> {
    let mut env =
        Environment::new(PROGRAM_ID, "bangk_stable", processor!(process_instruction)).await;
    let api_key = env
        .wallets
        .get("API")
        .ok_or("no API key in the environment")?;

    let admin1 = Pubkey::new_unique();
    let admin2 = Pubkey::new_unique();
    let admin3 = Pubkey::new_unique();

    let instruction = initialize(
        &api_key.pubkey(),
        &api_key.pubkey(),
        &admin1,
        &admin2,
        &admin3,
        &admin3,
    );
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert_eq!(
        res,
        Err(BangkError::DuplicatedKeyInMultisigDefinition),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn add_freeze_auth() -> Result<()> {
    let mut env = common::init_default().await?;

    let (freeze_pda, _) = MultiSigPda::get_address(MultiSigType::Freeze, &env.program_id);
    let freeze_old: MultiSigPda = env
        .from_account(&freeze_pda)
        .await
        .ok_or("could not load the freeze multisig")?;
    assert_eq!(freeze_old.multisig.sig_type, MultiSigType::Freeze);
    assert!(freeze_old.multisig.keys.is_empty(),);

    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    let freeze1 = env.wallets[FREEZE_USER1].pubkey();
    let freeze2 = env.wallets[FREEZE_USER2].pubkey();

    let freeze_new: MultiSigPda = env
        .from_account(&freeze_pda)
        .await
        .ok_or("could not load the freeze multisig")?;
    assert_eq!(freeze_new.multisig.sig_type, MultiSigType::Freeze);
    assert_eq!(freeze_new.multisig.keys, &[freeze1, freeze2]);

    Ok(())
}

#[tokio::test]
async fn add_duplicate_freeze_auth() -> Result<()> {
    let mut env = common::init_default().await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    let res = add_freeze_key(&mut env, FREEZE_USER1).await;
    assert_eq!(
        res,
        Err(BangkError::DuplicatedKeyInMultisigDefinition),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn remove_freeze_auth() -> Result<()> {
    let mut env = common::init_default().await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    remove_freeze_key(&mut env, FREEZE_USER1).await?;

    let freeze2 = env.wallets[FREEZE_USER2].pubkey();
    let (freeze_pda, _) = MultiSigPda::get_address(MultiSigType::Freeze, &env.program_id);
    let freeze_new: MultiSigPda = env
        .from_account(&freeze_pda)
        .await
        .ok_or("could not load the freeze multisig")?;
    assert_eq!(freeze_new.multisig.keys, &[freeze2]);

    Ok(())
}

#[tokio::test]
async fn remove_unknown_freeze() -> Result<()> {
    let mut env = common::init_default().await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    let _ = env.add_wallet("NOONE").await;
    let res = remove_freeze_key(&mut env, "NOONE").await;

    assert_eq!(res, Err(BangkError::InvalidOperation));

    Ok(())
}

#[tokio::test]
async fn freeze_thaw() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    mint_coins(&mut env, SYMBOL, "User 1", 10.0).await?;
    mint_coins(&mut env, SYMBOL, "User 2", 0.0).await?;
    freeze_ata(&mut env, "User 1", SYMBOL).await?;

    let res1 = mint_coins(&mut env, SYMBOL, "User 1", 1.0).await;
    assert_eq!(res1, Err(BangkError::InvalidFreezeStatus));
    let res2 = close_account(&mut env, SYMBOL, "User 1").await;
    assert_eq!(res2, Err(BangkError::InvalidFreezeStatus)); // that check is done before the amount of tokens
    let res3 = transfer_coins(&mut env, SYMBOL, "User 1", "User 2", 5.0).await;
    assert_eq!(res3, Err(BangkError::InvalidFreezeStatus));

    thaw_ata(&mut env, "User 1", SYMBOL).await?;
    mint_coins(&mut env, SYMBOL, "User 1", 10.0).await?;

    Ok(())
}

#[tokio::test]
async fn unauthorized_freeze() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let _ = env.add_wallet("Fake User").await;
    add_freeze_key(&mut env, "Fake User").await?;

    mint_coins(&mut env, SYMBOL, "User 1", 10.0).await?;
    let res = freeze_ata(&mut env, "User 1", SYMBOL).await;
    assert_eq!(res, Err(BangkError::InvalidSigner));

    Ok(())
}

#[tokio::test]
async fn double_freeze() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    mint_coins(&mut env, SYMBOL, "User 1", 10.0).await?;
    freeze_ata(&mut env, "User 1", SYMBOL).await?;
    let res1 = freeze_ata(&mut env, "User 1", SYMBOL).await;
    assert_eq!(res1, Err(BangkError::InvalidFreezeStatus));

    // And we still can’t mint of course
    let res2 = mint_coins(&mut env, SYMBOL, "User 1", 10.0).await;
    assert_eq!(res2, Err(BangkError::InvalidFreezeStatus));

    Ok(())
}

#[tokio::test]
async fn thaw_not_frozen() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    add_freeze_key(&mut env, FREEZE_USER1).await?;
    add_freeze_key(&mut env, FREEZE_USER2).await?;

    mint_coins(&mut env, SYMBOL, "User 1", 10.0).await?;
    let res1 = thaw_ata(&mut env, "User 1", SYMBOL).await;
    assert_eq!(res1, Err(BangkError::InvalidFreezeStatus));

    Ok(())
}
