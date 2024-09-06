// File: bangk-ico/tests/initialize.rs
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
#![allow(clippy::print_stdout)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result};

pub mod common;

use bangk_ico::{initialize, process_instruction, ConfigurationPda};
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use common::init_default;
use solana_program_test::{processor, tokio};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use tests_utilities::onchain::Environment;

use crate::common::{get_unvesting_def, PROGRAM_ID};

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
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
    )?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    let (config_pda, _) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (admin_pda, _) = MultiSigPda::get_address(MultiSigType::Admin, &env.program_id);

    // Testing configuration PDA integrity
    let config: ConfigurationPda = env
        .from_account(&config_pda)
        .await
        .ok_or("could not load the ICO program configuration")?;
    assert_eq!(config.unvesting.len(), 6);
    assert_eq!(
        config.admin_multisig, admin_pda,
        "error in the address for the admin PDA"
    );
    for def in get_unvesting_def() {
        assert!(config.unvesting.contains_key(&def.kind));
        assert!(
            config
                .unvesting
                .get(&def.kind)
                .is_some_and(|value| *value == def),
            "error in the definition of {:?}",
            def.kind
        );
    }

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
async fn wrong_signer() -> Result<()> {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;

    let random = env.add_wallet("random").await;

    println!("Initializing program");
    let instruction = initialize(
        &random,
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
    )?;
    let res = env.execute_transaction(&[instruction], &["random"]).await;
    println!("{res:?}");
    assert!(res.is_err_and(|err| err == BangkError::InvalidSigner));

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
    )?;
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_err_and(|err| err == BangkError::UniqueOperationAlreadyExecuted),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn duplicated_key_in_multisig() -> Result<()> {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
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
    )?;
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_err_and(|err| err == BangkError::DuplicatedKeyInMultisigDefinition),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}
