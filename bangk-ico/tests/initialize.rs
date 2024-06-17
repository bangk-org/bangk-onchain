// File: tests-onchain-ico/tests/initialize.rs
// Project: bangk-onchain
// Creation date: Thursday 13 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 25 July 2024 @ 22:11:54
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic)]
#![allow(clippy::print_stdout)]

pub mod common;

use bangk_ico::{
    initialize, process_instruction, ConfigurationPda, UnvestingScheme, UnvestingType,
};
use bangk_onchain_common::{
    security::{MultiSigPda, MultiSigType},
    Error,
};
use common::init_default;
use solana_program_test::{processor, tokio};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use tests_utilities::onchain::Environment;

use crate::common::{get_unvesting_def, PROGRAM_ID};

#[tokio::test]
async fn default() {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };
    let api_pub = api_key.pubkey();

    let admin1 = Pubkey::new_unique();
    let admin2 = Pubkey::new_unique();
    let admin3 = Pubkey::new_unique();
    let admin4 = Pubkey::new_unique();

    let Ok(instruction) = initialize(
        &api_key.pubkey(),
        get_unvesting_def(),
        &api_key.pubkey(),
        &admin1,
        &admin2,
        &admin3,
        &admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_ok(),
        "there was an unexpected error in the instruction"
    );

    let (config_pda, _) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (admin_pda, _) = MultiSigPda::get_address(MultiSigType::Admin, &env.program_id);

    // Testing configuration PDA integrity
    let Some(config): Option<ConfigurationPda> = env.from_account(&config_pda).await else {
        panic!("could not load the ICO program configuration");
    };
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
    let Some(admin): Option<MultiSigPda> = env.from_account(&admin_pda).await else {
        panic!("could not load the admin multisig");
    };
    assert_eq!(admin.multisig.sig_type, MultiSigType::Admin);
    assert_eq!(
        admin.multisig.keys,
        &[api_pub, admin1, admin2, admin3, admin4]
    );
}

#[tokio::test]
async fn wrong_signer() {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;

    let random = env.add_wallet("random").await;

    println!("Initializing program");
    let Ok(instruction) = initialize(
        &random,
        vec![],
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["random"]).await;
    println!("{res:?}");
    assert!(res.is_err_and(|err| err == Error::InvalidSigner));
}

#[tokio::test]
async fn invalid_investment_definition() {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };

    let Ok(instruction) = initialize(
        &api_key.pubkey(),
        vec![],
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(res.is_err_and(|err| err == Error::InvalidUnvestingDefinition));
}

#[tokio::test]
async fn duplicate_def() {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };

    let unvesting_def = vec![
        UnvestingScheme {
            kind: UnvestingType::TeamFounders,
            start: 52,
            duration: 157,
            initial_unvesting: 10000,
            weekly_unvesting: 800,
            final_unvesting: 6800,
        },
        UnvestingScheme {
            kind: UnvestingType::AdvisersPartners,
            start: 26,
            duration: 52,
            initial_unvesting: 10000,
            weekly_unvesting: 3500,
            final_unvesting: 2500,
        },
        UnvestingScheme {
            kind: UnvestingType::PrivateSells,
            start: 2,
            duration: 41,
            initial_unvesting: 10000,
            weekly_unvesting: 2300,
            final_unvesting: 2600,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells1,
            start: 2,
            duration: 41,
            initial_unvesting: 10000,
            weekly_unvesting: 2300,
            final_unvesting: 2600,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells2,
            start: 2,
            duration: 28,
            initial_unvesting: 10000,
            weekly_unvesting: 3500,
            final_unvesting: 2500,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells2,
            start: 2,
            duration: 15,
            initial_unvesting: 10000,
            weekly_unvesting: 7000,
            final_unvesting: 6000,
        },
    ];

    let Ok(instruction) = initialize(
        &api_key.pubkey(),
        unvesting_def,
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(res.is_err_and(|err| err == Error::InvalidUnvestingDefinition));
}

#[tokio::test]
async fn invalid_def() {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };

    let unvesting_def = vec![
        UnvestingScheme {
            kind: UnvestingType::TeamFounders,
            start: 52,
            duration: 157,
            initial_unvesting: 10000,
            weekly_unvesting: 800,
            final_unvesting: 6800,
        },
        UnvestingScheme {
            kind: UnvestingType::AdvisersPartners,
            start: 26,
            duration: 52,
            initial_unvesting: 10000,
            weekly_unvesting: 3500,
            final_unvesting: 2500,
        },
        UnvestingScheme {
            kind: UnvestingType::PrivateSells,
            start: 2,
            duration: 41,
            initial_unvesting: 10000,
            weekly_unvesting: 2300,
            final_unvesting: 2600,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells1,
            start: 2,
            duration: 41,
            initial_unvesting: 10000,
            weekly_unvesting: 2300,
            final_unvesting: 2600,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells2,
            start: 2,
            duration: 28,
            initial_unvesting: 10000,
            weekly_unvesting: 3500,
            final_unvesting: 2500,
        },
        UnvestingScheme {
            kind: UnvestingType::PublicSells3,
            start: 2,
            duration: 0,
            initial_unvesting: 0,
            weekly_unvesting: 0,
            final_unvesting: 0,
        },
    ];

    let Ok(instruction) = initialize(
        &api_key.pubkey(),
        unvesting_def,
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
        &Pubkey::new_unique(),
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(res.is_err_and(|err| err == Error::InvalidUnvestingDefinition));
}

#[tokio::test]
async fn double_init() {
    let mut env = init_default().await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };
    let admin1 = Pubkey::new_unique();
    let admin2 = Pubkey::new_unique();
    let admin3 = Pubkey::new_unique();
    let admin4 = Pubkey::new_unique();

    let Ok(instruction) = initialize(
        &api_key.pubkey(),
        get_unvesting_def(),
        &api_key.pubkey(),
        &admin1,
        &admin2,
        &admin3,
        &admin4,
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_err_and(|err| err == Error::UniqueOperationAlreadyExecuted),
        "there was an unexpected error in the instruction"
    );
}

#[tokio::test]
async fn duplicated_key_in_multisig() {
    let mut env = Environment::new(PROGRAM_ID, "bangk_ico", processor!(process_instruction)).await;
    let Some(api_key) = env.wallets.get("API") else {
        panic!("no API key in the environment");
    };

    let admin1 = Pubkey::new_unique();
    let admin2 = Pubkey::new_unique();
    let admin3 = Pubkey::new_unique();

    let Ok(instruction) = initialize(
        &api_key.pubkey(),
        get_unvesting_def(),
        &api_key.pubkey(),
        &admin1,
        &admin2,
        &admin3,
        &admin3,
    ) else {
        panic!("could not create instruction");
    };
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.is_err_and(|err| err == Error::DuplicatedKeyInMultisigDefinition),
        "there was an unexpected error in the instruction"
    );
}
