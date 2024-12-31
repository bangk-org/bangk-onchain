// File: bangk-stable/tests/transfers.rs
// Project: bangk-onchain
// Creation date: Monday 23 December 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Tuesday 31 December 2024 @ 16:27:44
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

use bangk_onchain_common::Error as BangkError;
use common::{get_ata, mint_coins, transfer_coins};
pub mod common;

const CURRENCY: &str = "Euro BANGK";
const SYMBOL: &str = "EUB";
const URI: &str = "https://api.bangk.app/token-eub";
const DECIMALS: u8 = 2;
const AMOUNT: f64 = 1000.;

const USER_SOURCE: &str = "User 1";
const USER_TARGET: &str = "User 2";

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, USER_SOURCE, AMOUNT).await?;
    mint_coins(&mut env, SYMBOL, USER_TARGET, AMOUNT).await?;

    let source_ata = get_ata(&env, USER_SOURCE, SYMBOL);
    let target_ata = get_ata(&env, USER_TARGET, SYMBOL);

    let expected = AMOUNT as u64 * 10_u64.pow(u32::from(DECIMALS));
    assert_eq!(
        env.get_token_amount(&source_ata)
            .await
            .ok_or("could not retrieve the token amount")?,
        expected
    );
    assert_eq!(
        env.get_token_amount(&target_ata)
            .await
            .ok_or("could not retrieve the token amount")?,
        expected
    );

    transfer_coins(&mut env, SYMBOL, "User 1", "User 2", AMOUNT).await?;
    assert_eq!(
        env.get_token_amount(&source_ata)
            .await
            .ok_or("could not retrieve the token amount")?,
        0_u64
    );
    assert_eq!(
        env.get_token_amount(&target_ata)
            .await
            .ok_or("could not retrieve the token amount")?,
        expected * 2
    );

    Ok(())
}

#[tokio::test]
async fn transfer_to_nonexisting_ata() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    let res = transfer_coins(&mut env, SYMBOL, "User 1", "User 2", AMOUNT).await;

    assert_eq!(
        res,
        Err(BangkError::ATADoesNotExist),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn transfer_invalid_amount() -> Result<()> {
    let mut env = common::init_with_mint(CURRENCY, SYMBOL, URI, DECIMALS).await?;

    mint_coins(&mut env, SYMBOL, "User 1", 10.0).await?;
    mint_coins(&mut env, SYMBOL, "User 2", 0.0).await?;
    let res = transfer_coins(&mut env, SYMBOL, "User 1", "User 2", -3.0).await;

    assert_eq!(
        res,
        Err(BangkError::InvalidAmount),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}
