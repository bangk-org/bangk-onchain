// File: bangk-ico/tests/investment.rs
// Project: bangk-onchain
// Creation date: Monday 17 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Thursday 22 August 2024 @ 13:08:32
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::panic_in_result_fn)]

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

use std::{error, result, thread::sleep, time::Duration};

use bangk_ico::{
    process_adviser_post_launch_investment, queue_adviser_post_launch_investment, user_investment,
    BangkIcoInstruction, ConfigurationPda, TimelockPda, UnvestingScheme, UnvestingType,
    UserInvestmentArgs, UserInvestmentPda, TIMELOCK_DELAY,
};
use bangk_onchain_common::{
    pda::PdaType,
    security::{MultiSigPda, MultiSigType},
    Error as BangkError,
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    system_program,
};
use solana_program_test::tokio;
use solana_sdk::{pubkey::Pubkey, signer::Signer as _};

use crate::common::{launch_tokens, PROGRAM_ID};

pub mod common;

const INVESTED_AMOUNT: u64 = 1_000_000_000;
const TOO_MANY: u64 = 51_000_000_000_000;

#[tokio::test]
async fn default() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let instruction = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(pda.pda_type, PdaType::IcoInvestment);
    assert_eq!(pda.investment.user, user);
    assert_eq!(pda.investment.investments.len(), 1);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);

    Ok(())
}

#[tokio::test]
async fn add_investment() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let instruction = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction.clone()], &["API"])
        .await?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
    assert_eq!(
        pda.investment.investments[1].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[1].custom_rule, None);
    assert_eq!(pda.investment.investments[1].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[1].amount_released, 0);

    Ok(())
}

#[tokio::test]
async fn two_investments1() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction1.clone()], &["API"])
        .await?;

    let instruction2 = user_investment(
        &api,
        &user,
        UnvestingType::AdvisersPartners,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction2], &["API"]).await?;

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::TeamFounders
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
    assert_eq!(
        pda.investment.investments[1].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(pda.investment.investments[1].custom_rule, None);
    assert_eq!(pda.investment.investments[1].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[1].amount_released, 0);

    Ok(())
}

#[tokio::test]
async fn two_investments2() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::PublicSells2,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction1.clone()], &["API"])
        .await?;

    let instruction2 = user_investment(
        &api,
        &user,
        UnvestingType::PublicSells3,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction2], &["API"]).await?;

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(pda.investment.investments.len(), 2);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::PublicSells2
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);
    assert_eq!(
        pda.investment.investments[1].kind,
        UnvestingType::PublicSells3
    );
    assert_eq!(pda.investment.investments[1].custom_rule, None);
    assert_eq!(pda.investment.investments[1].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[1].amount_released, 0);

    Ok(())
}

#[tokio::test]
async fn custom_scheme() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let custom_scheme = UnvestingScheme {
        kind: UnvestingType::AdvisersPartners,
        start: 10,
        duration: 12,
        initial_unvesting: 40_000,
        weekly_unvesting: 40_000,
        final_unvesting: 20_000,
    };

    let instruction = user_investment(
        &api,
        &user,
        UnvestingType::AdvisersPartners,
        Some(custom_scheme),
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction], &["API"]).await?;

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);
    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;
    assert_eq!(pda.pda_type, PdaType::IcoInvestment);
    assert_eq!(pda.investment.user, user);
    assert_eq!(pda.investment.investments.len(), 1);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(
        pda.investment.investments[0].custom_rule,
        Some(custom_scheme)
    );
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);

    Ok(())
}

#[tokio::test]
async fn invalid_custom_scheme() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let custom_scheme1 = UnvestingScheme {
        kind: UnvestingType::AdvisersPartners,
        start: 10,
        duration: 12,
        initial_unvesting: 40_000,
        weekly_unvesting: 40_000,
        final_unvesting: 10_000,
    };

    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::TeamFounders,
        Some(custom_scheme1),
        INVESTED_AMOUNT,
    )?;
    let res1 = env.execute_transaction(&[instruction1], &["API"]).await;
    assert!(
        res1.is_err_and(|err| err == BangkError::InvalidUnvestingDefinition),
        "there was an unexpected error in the instruction"
    );

    let custom_scheme2 = UnvestingScheme {
        kind: UnvestingType::AdvisersPartners,
        start: 10,
        duration: 12,
        initial_unvesting: 30_000,
        weekly_unvesting: 40_000,
        final_unvesting: 10_000,
    };

    let instruction2 = user_investment(
        &api,
        &user,
        UnvestingType::AdvisersPartners,
        Some(custom_scheme2),
        INVESTED_AMOUNT,
    )?;
    let res2 = env.execute_transaction(&[instruction2], &["API"]).await;
    assert!(
        res2.is_err_and(|err| err == BangkError::InvalidUnvestingDefinition),
        "there was an unexpected error in the instruction"
    );

    Ok(())
}

#[tokio::test]
async fn too_many() -> Result<()> {
    let mut env = common::init_default().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let instruction = user_investment(&api, &user, UnvestingType::TeamFounders, None, TOO_MANY)?;
    let res = env.execute_transaction(&[instruction], &["API"]).await;
    assert!(
        res.as_ref()
            .is_err_and(|err| *err == BangkError::InvalidAmount),
        "res: {res:?}"
    );

    Ok(())
}

#[tokio::test]
async fn post_launch_advisers_investment() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = Pubkey::new_unique();

    launch_tokens(&mut env, chrono::Utc::now().timestamp() - 4 * 7 * 24 * 3600).await?;

    let instruction = queue_adviser_post_launch_investment(
        &admin1,
        &admin2,
        &admin3,
        &user,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction2 = process_adviser_post_launch_investment(&api, &user, None, INVESTED_AMOUNT)?;
    env.execute_transaction(&[instruction2], &["API"]).await?;

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);

    let pda: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;

    assert_eq!(pda.investment.user, user);
    assert_eq!(pda.investment.investments.len(), 1);
    assert_eq!(
        pda.investment.investments[0].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(pda.investment.investments[0].custom_rule, None,);
    assert_eq!(pda.investment.investments[0].amount_bought, INVESTED_AMOUNT);
    assert_eq!(pda.investment.investments[0].amount_released, 0);

    Ok(())
}

#[tokio::test]
async fn post_launch_advisers_investment_before_launch() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = Pubkey::new_unique();

    let instruction = queue_adviser_post_launch_investment(
        &admin1,
        &admin2,
        &admin3,
        &user,
        None,
        INVESTED_AMOUNT,
    )?;
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(res.is_err_and(|err| err == BangkError::PostLaunchInvestmentBeforeLaunch));

    Ok(())
}

#[allow(clippy::missing_errors_doc)]
pub fn custom_non_adviser_post_launch(
    admin1: &Pubkey,
    admin2: &Pubkey,
    admin3: &Pubkey,
    user: &Pubkey,
    custom_rule: Option<UnvestingScheme>,
    amount: u64,
) -> result::Result<Instruction, ProgramError> {
    let (config_pda, _config_bump) = ConfigurationPda::get_address(&bangk_ico::ID);
    let (admin_keys_pda, _admin_bump) =
        MultiSigPda::get_address(MultiSigType::Admin, &bangk_ico::ID);
    let (timelock_pda, _timelock_bump) = TimelockPda::get_address(&bangk_ico::ID);

    Ok(Instruction {
        program_id: bangk_ico::ID,
        accounts: vec![
            AccountMeta::new(*admin1, true),
            AccountMeta::new_readonly(*admin2, true),
            AccountMeta::new_readonly(*admin3, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(admin_keys_pda, false),
            AccountMeta::new(timelock_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&BangkIcoInstruction::QueuePostLaunchAdvisersInvestment(
            UserInvestmentArgs {
                user: *user,
                invest_kind: UnvestingType::TeamFounders,
                custom_rule,
                amount,
            },
        ))?,
    })
}

#[tokio::test]
async fn post_launch_advisers_investment_non_adviser() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = Pubkey::new_unique();

    launch_tokens(&mut env, chrono::Utc::now().timestamp() - 4 * 7 * 24 * 3600).await?;

    let instruction =
        custom_non_adviser_post_launch(&admin1, &admin2, &admin3, &user, None, INVESTED_AMOUNT)?;
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(res.is_err_and(|err| err == BangkError::InvalidOperation));

    Ok(())
}

#[tokio::test]
async fn add_investment_post_launch() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let user = Pubkey::new_unique();

    let instruction1 = user_investment(
        &api,
        &user,
        UnvestingType::PublicSells1,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction1], &["API"]).await?;

    launch_tokens(&mut env, chrono::Utc::now().timestamp() - 4 * 7 * 24 * 3600).await?;

    let instruction2 = user_investment(
        &api,
        &user,
        UnvestingType::PublicSells1,
        None,
        INVESTED_AMOUNT,
    )?;
    let res = env
        .execute_transaction(&[instruction2.clone()], &["API"])
        .await;
    assert!(res.is_err_and(|err| err == BangkError::IcoInvestAfterLaunch));

    Ok(())
}

#[tokio::test]
async fn post_launch_advisers_investment_too_many() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = Pubkey::new_unique();

    launch_tokens(&mut env, chrono::Utc::now().timestamp() - 4 * 7 * 24 * 3600).await?;

    let instruction =
        queue_adviser_post_launch_investment(&admin1, &admin2, &admin3, &user, None, TOO_MANY)?;
    let res = env
        .execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await;
    assert!(
        res.as_ref()
            .is_err_and(|err| *err == BangkError::InvalidAmount),
        "res: {res:?}"
    );

    Ok(())
}

#[tokio::test]
async fn post_launch_advisers_investment_double() -> Result<()> {
    let mut env = common::init_with_mint().await?;

    let api = env.wallets["API"].pubkey();
    let admin1 = env.wallets["Admin 1"].pubkey();
    let admin2 = env.wallets["Admin 2"].pubkey();
    let admin3 = env.wallets["Admin 3"].pubkey();
    let user = Pubkey::new_unique();

    let (investment_pda, _investment_bump) = UserInvestmentPda::get_address(user, &PROGRAM_ID);

    launch_tokens(&mut env, chrono::Utc::now().timestamp() - 4 * 7 * 24 * 3600).await?;

    let instruction = queue_adviser_post_launch_investment(
        &admin1,
        &admin2,
        &admin3,
        &user,
        None,
        INVESTED_AMOUNT,
    )?;
    env.execute_transaction(&[instruction.clone()], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;
    env.execute_transaction(&[instruction], &["Admin 1", "Admin 2", "Admin 3"])
        .await?;
    // Wait for the timeout
    sleep(Duration::from_secs(TIMELOCK_DELAY as u64));
    // Execute the instruction
    let instruction2 = process_adviser_post_launch_investment(&api, &user, None, INVESTED_AMOUNT)?;
    env.execute_transaction(&[instruction2.clone()], &["API"])
        .await?;

    // Check that only one of the two was processed
    let pda1: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;

    assert_eq!(pda1.investment.user, user);
    assert_eq!(pda1.investment.investments.len(), 1);
    assert_eq!(
        pda1.investment.investments[0].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(pda1.investment.investments[0].custom_rule, None,);
    assert_eq!(
        pda1.investment.investments[0].amount_bought,
        INVESTED_AMOUNT
    );
    assert_eq!(pda1.investment.investments[0].amount_released, 0);

    // Check that both have been processed
    env.execute_transaction(&[instruction2], &["API"]).await?;
    let pda2: UserInvestmentPda = env
        .from_account(&investment_pda)
        .await
        .ok_or("could not load the investment PDA")?;

    assert_eq!(pda2.investment.user, user);
    assert_eq!(pda2.investment.investments.len(), 2);
    assert_eq!(
        pda2.investment.investments[1].kind,
        UnvestingType::AdvisersPartners
    );
    assert_eq!(pda2.investment.investments[1].custom_rule, None,);
    assert_eq!(
        pda2.investment.investments[1].amount_bought,
        INVESTED_AMOUNT
    );
    assert_eq!(pda2.investment.investments[1].amount_released, 0);

    Ok(())
}
