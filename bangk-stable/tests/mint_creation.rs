// File: bangk-stable/tests/mint_creation.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 31 December 2024 @ 16:46:37
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
use bangk_stable::{create_stable_coin, get_stable_coin_mint, update_stable_coin};
use common::{create_coin, get_exchange};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

const CURRENCY: &str = "Euro BANGK";
const SYMBOL: &str = "EUB";
const URI: &str = "https://api.bangk.app/token-eub";
const DECIMALS: u8 = 2;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    let mint_address = get_stable_coin_mint(SYMBOL);
    let pda_exchange = get_exchange(SYMBOL);
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
    assert_eq!(tokens, Some(0));

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
    );
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "User"])
        .await;
    assert_eq!(res, Err(BangkError::InvalidSigner),);

    Ok(())
}

#[tokio::test]
async fn double_creation() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let res = create_coin(&mut env, CURRENCY, SYMBOL, URI, DECIMALS).await;
    assert_eq!(res, Err(BangkError::UniqueOperationAlreadyExecuted),);

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
    let mint = get_stable_coin_mint(SYMBOL);

    let instruction = update_stable_coin(
        &admin1,
        &admin2,
        &admin3,
        &mint,
        Some(new_name.to_owned()),
        Some(new_uri.to_owned()),
    );
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;

    // Checking mint
    let metadata = env
        .get_mint_metadata(&mint)
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
    let mint = get_stable_coin_mint(SYMBOL);
    let instruction = update_stable_coin(&admin1, &admin2, &admin3, &mint, None, None);
    // println!("Instruction: {instruction:#?}");
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert_eq!(res, Err(BangkError::InvalidOperation),);

    Ok(())
}

#[tokio::test]
async fn update_nonexisting_mint() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;
    let new_name = "USB".to_owned();
    let new_uri = "new_uri".to_owned();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let mint = get_stable_coin_mint("JPB");
    let instruction = update_stable_coin(
        &admin1,
        &admin2,
        &admin3,
        &mint,
        Some(new_name),
        Some(new_uri),
    );
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert_eq!(res, Err(BangkError::InvalidPdaAddress),);

    Ok(())
}
