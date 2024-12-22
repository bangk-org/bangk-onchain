// File: bangk-stable/tests/mint_creation.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Sunday 22 December 2024 @ 18:55:18
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic_in_result_fn)]
#![allow(clippy::print_stdout)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

pub mod common;
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use bangk_stable::{create_stable_coin, mint, update_stable_coin, EXCHANGE_SEED, STABLE_MINT_SEED};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

const CURRENCY: &str = "Euro BANGK";
const SYMBOL: &str = "EUB";
const URI: &str = "https://api.bangk.app/token-eub";
const DECIMALS: u8 = 2;
const AMOUNT: f64 = 1000.;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    let mint_address = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), SYMBOL.as_bytes()],
        &bangk_stable::ID,
    )
    .0;
    let pda_exchange = Pubkey::find_program_address(
        &[EXCHANGE_SEED.as_bytes(), &mint_address.to_bytes()],
        &bangk_stable::ID,
    )
    .0;
    println!("{mint_address}");
    let (admin_pda, _) = MultiSigPda::get_address(MultiSigType::Admin, &env.program_id);

    // Checking mint
    let mint = env.get_mint_state(&mint_address).await;
    assert_eq!(mint.supply, 0);
    assert_eq!(mint.decimals, DECIMALS);
    assert_eq!(mint.freeze_authority, Some(admin_pda).into());
    assert_eq!(mint.mint_authority, Some(admin_pda).into());
    println!("{mint:#?}");
    let metadata = env
        .get_mint_metadata(&mint_address)
        .await
        .ok_or("could not get mint metadata")?;
    println!("{metadata:#?}");
    assert_eq!(metadata.name, CURRENCY);
    assert_eq!(metadata.symbol, SYMBOL);
    assert_eq!(metadata.uri, URI);
    assert_eq!(metadata.update_authority, Some(admin_pda).try_into()?);

    // Chechking exchange wallets
    let pda = env.get_account_state(&pda_exchange).await;
    assert_eq!(pda.mint, mint_address);
    assert_eq!(pda.owner, admin_pda);
    assert_eq!(pda.delegate, None.into());
    let tokens = env.get_token_amount(&pda_exchange).await;
    assert!(tokens.is_some_and(|toks| toks == 0));

    Ok(())
}

#[tokio::test]
async fn wrong_signer() -> Result<()> {
    let mut env = common::init_default().await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let user = env.add_wallet("User").await;
    let instruction = create_stable_coin(
        &admin1,
        &admin2,
        &user,
        CURRENCY.to_owned(),
        SYMBOL.to_owned(),
        URI.to_owned(),
        DECIMALS,
    )?;
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "User"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::InvalidSigner),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn double_creation() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction = create_stable_coin(
        &admin1,
        &admin2,
        &admin3,
        CURRENCY.to_owned(),
        SYMBOL.to_owned(),
        URI.to_owned(),
        DECIMALS,
    )?;
    // println!("Instruction: {instruction:#?}");
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::UniqueOperationAlreadyExecuted),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn update_metadata() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let new_name = "USB";
    let new_uri = "new_uri";
    let instruction = update_stable_coin(
        &admin1,
        &admin2,
        &admin3,
        SYMBOL.to_owned(),
        Some(new_name.to_owned()),
        Some(new_uri.to_owned()),
    )?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    let mint_address = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), SYMBOL.as_bytes()],
        &bangk_stable::ID,
    )
    .0;
    // Checking mint
    let metadata = env
        .get_mint_metadata(&mint_address)
        .await
        .ok_or("could not get mint metadata")?;
    println!("{metadata:#?}");
    assert_eq!(metadata.name, new_name);
    assert_eq!(metadata.uri, new_uri);

    Ok(())
}

#[tokio::test]
async fn update_metadata_nodata() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction = update_stable_coin(&admin1, &admin2, &admin3, SYMBOL.to_owned(), None, None)?;
    // println!("Instruction: {instruction:#?}");
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_err_and(|err| err == BangkError::InvalidOperation),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn mint_operation() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let user = env.add_wallet("User 1").await;
    let instruction = mint(&admin1, &user, SYMBOL, AMOUNT)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction], &["Admin 1"])
        .await?;

    let mint_address = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), SYMBOL.as_bytes()],
        &bangk_stable::ID,
    )
    .0;
    let ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::id());
    let amount = env
        .get_token_amount(&ata)
        .await
        .ok_or("could not retrieve the token amount")?;
    let expected = AMOUNT as u64 * 10_u64.pow(u32::from(DECIMALS));
    assert_eq!(expected, amount);

    Ok(())
}

#[tokio::test]
async fn mint_operation_existing_ata() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let user = env.add_wallet("User 2").await;
    let instruction1 = mint(&admin1, &user, SYMBOL, AMOUNT)?;
    // println!("Instruction: {instruction:#?}");
    env.execute_transaction(&[instruction1], &["Admin 1"])
        .await?;
    // Double, this time the ATA exists
    let instruction2 = mint(&admin1, &user, SYMBOL, AMOUNT)?;
    env.execute_transaction(&[instruction2], &["Admin 1"])
        .await?;

    let mint_address = Pubkey::find_program_address(
        &[STABLE_MINT_SEED.as_bytes(), SYMBOL.as_bytes()],
        &bangk_stable::ID,
    )
    .0;
    let ata =
        get_associated_token_address_with_program_id(&user, &mint_address, &spl_token_2022::id());
    let amount = env
        .get_token_amount(&ata)
        .await
        .ok_or("could not retrieve the token amount")?;
    let expected = AMOUNT as u64 * 2 * 10_u64.pow(u32::from(DECIMALS));
    assert_eq!(expected, amount);

    Ok(())
}
