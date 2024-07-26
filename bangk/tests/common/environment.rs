// File: tests-onchain-main/tests/common/environment.rs
// Project: bangk-onchain
// Creation date: Friday 26 July 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Friday 26 July 2024 @ 22:20:09
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

// File: tests-onchain-main/tests/common/environment.rs
// Project: bangk-onchain
// Creation date: Friday 24 November 2023
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 10 July 2024 @ 01:26:29
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

#![allow(dead_code)]

use std::collections::HashMap;

use bangk::{
    processor::process_instruction,
    state::projects::{Project, ProjectStatus},
};
use borsh::BorshSerialize;
use solana_program::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    system_program,
    {pubkey, pubkey::Pubkey},
};
use solana_program_test::{processor, BanksClient, ProgramTest, ProgramTestBanksClientExt};
use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};
use spl_token_2022::{
    extension::{
        metadata_pointer::MetadataPointer, BaseStateWithExtensions, Extension, StateWithExtensions,
    },
    solana_zk_token_sdk::zk_token_proof_instruction::Pod,
    state::{self, Mint},
};
use spl_token_metadata_interface::{borsh::BorshDeserialize, state::TokenMetadata};

pub struct Environment {
    pub program_id: Pubkey,
    pub wallets: HashMap<String, Keypair>,
    pub accounts: HashMap<String, (Pubkey, u8)>,
    pub client: BanksClient,
    pub payer: Keypair,
    pub delegate: Keypair,
    pub blockhash: Hash,
}

pub const PROGRAM_ID: Pubkey = pubkey!("BKPrg2BFZLMzLtujrsT7ayVewgVCGkKUwdB9e3E6Kzyp");
const TEST_KEY: [u8; 64] = [
    29, 238, 17, 250, 48, 124, 110, 93, 126, 238, 0, 241, 188, 40, 229, 185, 116, 45, 103, 72, 100,
    120, 126, 26, 191, 169, 241, 114, 185, 147, 230, 30, 241, 221, 196, 199, 134, 183, 206, 136,
    205, 162, 105, 186, 99, 228, 45, 248, 95, 176, 164, 34, 110, 163, 84, 179, 82, 240, 225, 185,
    112, 153, 240, 58,
];
const DELEGATE_KEY: [u8; 64] = [
    253, 46, 46, 53, 143, 3, 254, 37, 25, 240, 238, 41, 244, 42, 21, 53, 176, 200, 172, 41, 116,
    43, 97, 80, 192, 150, 162, 127, 184, 66, 51, 70, 164, 171, 175, 243, 36, 37, 70, 145, 191, 90,
    83, 99, 239, 76, 234, 89, 73, 42, 184, 249, 219, 238, 62, 137, 89, 165, 71, 211, 19, 40, 113,
    75,
];

impl Environment {
    pub async fn get() -> Self {
        let bangk = Keypair::from_bytes(&TEST_KEY).unwrap();
        let bangk_account = Account::new(5_000_000_000, 0, &system_program::ID);
        let delegate = Keypair::from_bytes(&DELEGATE_KEY).unwrap();
        let delegate_account = Account::new(5_000_000_000, 0, &system_program::ID);
        let mut program_test = ProgramTest::default();
        program_test.prefer_bpf(false);
        program_test.add_program("bangk", PROGRAM_ID, processor!(process_instruction));
        program_test.add_account(bangk.pubkey(), bangk_account);
        program_test.add_account(delegate.pubkey(), delegate_account);
        let (banks_client, _, recent_blockhash) = program_test.start().await;

        Self {
            program_id: PROGRAM_ID,
            wallets: HashMap::new(),
            accounts: HashMap::new(),
            client: banks_client,
            payer: bangk,
            delegate,
            blockhash: recent_blockhash,
        }
    }

    pub async fn execute_transaction(&mut self, instructions: &[Instruction], signer: &[&str]) {
        let mut signers = vec![&self.payer];
        if !signer.is_empty() {
            signer
                .iter()
                .filter_map(|s| {
                    if s.to_lowercase() == "bangk" {
                        // already added, ignore
                        None
                    } else if s.to_lowercase() == "delegate" {
                        Some(&self.delegate)
                    } else {
                        println!("getting wallet for {s}");
                        Some(&self.wallets[*s])
                    }
                })
                .for_each(|s| signers.push(s));
        }
        let mut transaction = Transaction::new_with_payer(instructions, Some(&self.payer.pubkey()));
        // println!(
        //     "Signers:\n{:#?}",
        //     signers
        //         .iter()
        //         .map(|&keypair| keypair.pubkey())
        //         .collect::<Vec<_>>()
        // );
        // println!(
        //     "Required signers:\n{:#?}",
        //     instructions
        //         .iter()
        //         .flat_map(|instr| instr
        //             .accounts
        //             .iter()
        //             .filter(|&account| account.is_signer)
        //             .map(|account| account.pubkey)
        //             .collect::<Vec<_>>())
        //         .collect::<Vec<_>>()
        // );
        // println!("Transaction:\n{:#?}", transaction);
        transaction.sign(signers.as_slice(), self.blockhash);
        self.client.process_transaction(transaction).await.unwrap();

        // Go to the next blockhash to prevent duplicated transactions from being ignored
        self.blockhash = self
            .client
            .get_new_latest_blockhash(&self.blockhash)
            .await
            .unwrap();
    }

    pub async fn execute_failing_transaction(
        &mut self,
        payload: &impl BorshSerialize,
        accounts: &[AccountMeta],
        signer: &[&str],
    ) {
        let mut signers = Vec::new();
        if signer.is_empty() {
            signers.push(&self.payer);
        } else {
            signer
                .iter()
                .map(|s| {
                    if *s == "bangk" {
                        &self.payer
                    } else {
                        &self.wallets[*s]
                    }
                })
                .for_each(|s| signers.push(s));
        }

        let payload = borsh::to_vec(&payload).unwrap();
        let mut transaction = Transaction::new_with_payer(
            &[Instruction::new_with_bytes(
                self.program_id,
                &payload,
                accounts.to_owned(),
            )],
            Some(&self.payer.pubkey()),
        );
        transaction.sign(signers.as_slice(), self.blockhash);
        self.client.process_transaction(transaction).await.unwrap();

        // Go to the next blockhash to prevent duplicated transactions from being ignored
        self.blockhash = self
            .client
            .get_new_latest_blockhash(&self.blockhash)
            .await
            .unwrap();
    }

    pub async fn get_account(&mut self, address: Pubkey) -> Option<Account> {
        self.client.get_account(address).await.unwrap()
    }

    pub async fn get_token_amount(&mut self, address: Pubkey) -> Option<u64> {
        let res = self.get_account(address).await?;
        let state = StateWithExtensions::<state::Account>::unpack(&res.data)
            .unwrap()
            .base;
        Some(state.amount)
    }

    /// Get a mint base state
    ///
    /// # Parameters
    /// * `env` - Test environment
    /// * `address` - Pubkey of the mint
    ///
    /// # Returns
    /// - The mint's base state.
    pub async fn get_mint_state(&mut self, address: Pubkey) -> Mint {
        let res = self.get_account(address).await.unwrap();
        StateWithExtensions::<state::Mint>::unpack(&res.data)
            .unwrap()
            .base
    }

    pub async fn get_mint_state_with_extensions<T: Extension + Pod>(
        &mut self,
        address: Pubkey,
    ) -> T {
        let res = self.get_account(address).await.unwrap();
        let state = StateWithExtensions::<state::Mint>::unpack(&res.data).unwrap();
        *state.get_extension::<T>().unwrap()
    }

    pub async fn get_mint_metadata(&mut self, address: Pubkey) -> TokenMetadata {
        const META_START: usize = 279;
        let metadata = self
            .get_mint_state_with_extensions::<MetadataPointer>(address)
            .await;
        let key: Option<Pubkey> = metadata.metadata_address.into();
        assert_eq!(key.unwrap(), address);
        let data = self.get_account(key.unwrap()).await.unwrap();
        TokenMetadata::try_from_slice(&data.data[META_START..]).unwrap()
    }
    pub async fn get_project(&mut self, address: Pubkey) -> Project {
        let metadata = self.get_mint_metadata(address).await;
        let mut res = Project {
            name: metadata.name,
            symbol: metadata.symbol,
            uri: metadata.uri,
            ..Default::default()
        };

        for (key, value) in metadata.additional_metadata {
            match key.as_str() {
                "rate" => res.interest_rate = value.parse().unwrap(),
                "last" => res.last_payment = value.parse().unwrap(),
                "next" => res.next_payment = value.parse().unwrap(),
                "value" => res.token_value = value.parse().unwrap(),
                "risk" => res.risk_assessment = value.parse().unwrap(),
                "period" => {
                    res.payment_periodicity =
                        value.parse::<u8>().unwrap_or_default().try_into().unwrap();
                }
                "status" => {
                    res.status = value.parse::<u8>().unwrap_or_default().try_into().unwrap();
                }
                "ata" => {
                    res.ata = value;
                }
                _ => (),
            }
        }

        res
    }

    pub async fn get_project_status(&mut self, address: Pubkey) -> ProjectStatus {
        self.get_project(address).await.status
    }

    pub async fn get_project_next_payment(&mut self, address: Pubkey) -> i64 {
        self.get_project(address).await.next_payment
    }
}
