// File: tests-onchain-ico/tests/mint_creation.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 24 July 2024 @ 19:00:57
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

pub mod common;
use bangk_ico::create_mint;
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error,
};
use solana_program::program_option::COption;
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_associated_token_account::get_associated_token_address_with_program_id;

use crate::common::PROGRAM_ID;

#[tokio::test]
async fn default() {
    let mut env = common::init_with_mint().await;

    let (admin_keys_pda, _admin_bump) = MultiSigPda::get_address(MultiSigType::Admin, &PROGRAM_ID);
    let (mint_address, _mint_bump) = Pubkey::find_program_address(&[b"Mint", b"BGK"], &PROGRAM_ID);
    let reserve_ata = get_associated_token_address_with_program_id(
        &admin_keys_pda,
        &mint_address,
        &spl_token_2022::ID,
    );

    // Checking mint
    let mint = env.get_mint_state(&mint_address).await;
    assert_eq!(mint.supply, 177_000_000_000_000);
    assert_eq!(mint.decimals, 6);
    assert_eq!(mint.freeze_authority, COption::<Pubkey>::None);
    assert_eq!(mint.mint_authority, None.into());
    println!("{mint:#?}");
    let Some(metadata) = env.get_mint_metadata(&mint_address).await else {
        panic!("could not get mint metadata");
    };
    assert_eq!(metadata.name, "Bangk Coin");
    assert_eq!(metadata.symbol, "BGK");
    assert_eq!(metadata.uri, "https://bangk.app/bgk_token.json");

    // Chechking ATA
    let ata = env.get_account_state(&reserve_ata).await;
    assert_eq!(ata.mint, mint_address);
    assert_eq!(ata.owner, admin_keys_pda);
    assert_eq!(ata.delegate, None.into());
    let tokens = env.get_token_amount(&reserve_ata).await;
    assert!(tokens.is_some_and(|toks| toks == 177_000_000_000_000));
}

#[tokio::test]
async fn wrong_signer() {
    let mut env = common::init_default().await;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let user = env.add_wallet("User").await;
    let Ok(instruction) = create_mint(&admin1, &admin2, &user) else {
        panic!("could not create instruction");
    };
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "User"])
        .await;
    assert!(
        res.is_err_and(|err| err == Error::InvalidSigner),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn double_creation() {
    let mut env = common::init_with_mint().await;
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let Ok(instruction) = create_mint(&admin1, &admin2, &admin3) else {
        panic!("could not create instruction");
    };
    // println!("Instruction: {instruction:#?}");
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.is_err_and(|err| err == Error::UniqueOperationAlreadyExecuted),
        "there was an unexpected error in the instruction"
    );
}
