// File: bangk-ico/tests/mint_creation.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 12:25:59
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

pub mod common;
use bangk_ico::{create_mint, WALLET_INIT_AMOUNT};
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use solana_program::program_option::COption;
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use crate::common::PROGRAM_ID;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let (admin_pda, _) = MultiSigPda::get_address(MultiSigType::Admin, &env.program_id);

    // Checking mint
    let mint = env.get_mint_state(&mint_address).await;
    assert_eq!(mint.supply, 177_000_000_000_000);
    assert_eq!(mint.decimals, 6);
    assert_eq!(mint.freeze_authority, COption::<Pubkey>::None);
    assert_eq!(mint.mint_authority, None.into());
    println!("{mint:#?}");
    let metadata = env
        .get_mint_metadata(&mint_address)
        .await
        .ok_or("could not get mint metadata")?;
    assert_eq!(metadata.name, "Bangk Coin");
    assert_eq!(metadata.symbol, "BGK");
    assert_eq!(metadata.uri, "https://bangk.app/bgk_token.json");

    // Chechking wallets
    for (wallet, amount) in WALLET_INIT_AMOUNT {
        let pda_address = wallet.get_pda().0;
        let pda = env.get_account_state(&pda_address).await;
        assert_eq!(pda.mint, mint_address);
        assert_eq!(pda.owner, admin_pda);
        assert_eq!(pda.delegate, None.into());
        let tokens = env.get_token_amount(&pda_address).await;
        assert!(tokens.is_some_and(|toks| toks == amount.saturating_mul(1_000_000)));
    }

    Ok(())
}

#[tokio::test]
async fn wrong_signer() -> Result<()> {
    let mut env = common::init_default().await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let user = env.add_wallet("User").await;
    let instruction = create_mint(&admin1, &admin2, &user)?;
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
    let mut env = common::init_with_mint().await?;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let instruction = create_mint(&admin1, &admin2, &admin3)?;
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
