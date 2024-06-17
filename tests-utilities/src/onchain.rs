// File: tests-utilities/src/onchain.rs
// Project: bangk-solana
// Creation date: Sunday 09 June 2024
// Author: Vincent Berthier <vincent.berthier@bangk.fr>
// -----
// Last modified: Monday 24 June 2024 @ 20:35:38
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use std::{collections::HashMap, fmt::Debug};

use bangk_onchain_common::{errors::BangkError, pda::BangkPda};
use borsh::BorshDeserialize;
use solana_program::{
    hash::Hash, instruction::Instruction, pubkey::Pubkey, system_instruction::transfer,
    system_program,
};
use solana_program_runtime::invoke_context::BuiltinFunctionWithContext;
use solana_program_test::{BanksClient, BanksClientError, ProgramTest, ProgramTestBanksClientExt};
use solana_sdk::{
    account::Account,
    instruction::InstructionError,
    signature::{keypair_from_seed_phrase_and_passphrase, Keypair},
    signer::Signer,
    transaction::{Transaction, TransactionError},
};
use spl_token_2022::{
    extension::{
        metadata_pointer::MetadataPointer, BaseStateWithExtensions as _, Extension,
        StateWithExtensions,
    },
    solana_zk_token_sdk::instruction::Pod,
    state::{self, Mint},
};
use spl_token_metadata_interface::{borsh::BorshDeserialize as _, state::TokenMetadata};

const API_KEY: [u8; 64] = [
    29, 238, 17, 250, 48, 124, 110, 93, 126, 238, 0, 241, 188, 40, 229, 185, 116, 45, 103, 72, 100,
    120, 126, 26, 191, 169, 241, 114, 185, 147, 230, 30, 241, 221, 196, 199, 134, 183, 206, 136,
    205, 162, 105, 186, 99, 228, 45, 248, 95, 176, 164, 34, 110, 163, 84, 179, 82, 240, 225, 185,
    112, 153, 240, 58,
];

/// Environment used for On-Chain tests
pub struct Environment {
    /// Public key of the program
    pub program_id: Pubkey,
    /// Testing runtime / cluster
    pub client: BanksClient,
    /// Current block
    pub blockhash: Hash,
    /// Map of Name - Keypair for all wallets used in the tests
    pub wallets: HashMap<String, Keypair>,
    /// Map of Name - (Address, Bump) for all PDAs in the tests
    pub pda: HashMap<String, (Pubkey, u8)>,
    /// Map of (Name, Mint Address) - Address for all ATAs in the tests
    pub ata: HashMap<(String, Pubkey), Pubkey>,
}

impl Environment {
    /// Creates a new testing environment.
    ///
    /// # Parameters
    /// * `program_id` - Address of the program,
    /// * `entrypoint` - Program's entrypoint (call with `solana_program_test::processor!(entrypoint)`)
    ///
    /// # Panics
    /// If the environment couldn't be created (API key was not parsed successfully for example)
    pub async fn new(
        program_id: Pubkey,
        program: &str,
        entrypoint: Option<BuiltinFunctionWithContext>,
    ) -> Self {
        println!("Creating environment");
        let Ok(bangk_key) = Keypair::from_bytes(&API_KEY) else {
            panic!("could not parse API key while setting up environment");
        };
        let bangk_account = Account::new(50_000_000_000, 0, &system_program::ID);
        let mut program_test = ProgramTest::default();
        program_test.prefer_bpf(false);
        program_test.add_program(program, program_id, entrypoint);
        program_test.add_account(bangk_key.pubkey(), bangk_account);
        let (banks_client, _, recent_blockhash) = program_test.start().await;

        Self {
            program_id,
            client: banks_client,
            blockhash: recent_blockhash,
            wallets: HashMap::from([("API".to_owned(), bangk_key)]),
            pda: HashMap::new(),
            ata: HashMap::new(),
        }
    }

    /// Executes a transaction
    ///
    /// Once the transaction is finished, the block will be switched for a new one,
    /// which prevents duplicated instructions from being ignored.
    ///
    /// # Errors
    /// If an instruction errors and it's a `BangkError`, it is properly returned.
    ///
    /// # Panics
    /// If there is an error, but it's not a Custom one, then there's a panic as it shouldn't happen.
    /// Can also happen if there are no signers
    pub async fn execute_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: &[&str],
    ) -> Result<(), BangkError> {
        println!("Executing transaction");
        let signers: Vec<&Keypair> = signers
            .iter()
            .filter_map(|name| self.wallets.get(*name))
            .collect();
        assert!(!signers.is_empty(), "signers must not be empty");
        let mut transaction =
            Transaction::new_with_payer(instructions, Some(&signers.first().unwrap().pubkey()));
        transaction.sign(signers.as_slice(), self.blockhash);
        let res = self.client.process_transaction(transaction).await;

        // Go to the next blockhash to prevent duplicated transactions from being ignored
        self.blockhash = self
            .client
            .get_new_latest_blockhash(&self.blockhash)
            .await
            .unwrap();

        match res {
            Ok(()) => Ok(()),
            Err(BanksClientError::TransactionError(TransactionError::InstructionError(
                _num,
                InstructionError::Custom(err),
            ))) => Err(BangkError::from(err)),
            Err(err) => panic!("Unexpected error: {err}"),
        }
    }

    /// Get the state of an account.
    ///
    /// If the account doesn't exist, `None` will be returned.
    ///
    /// # Parameters
    /// * `address` - Address of the account for which to get the state
    ///
    /// # Panics
    /// If the account could not be retrieved (existing or not)
    pub async fn get_account(&mut self, address: &Pubkey) -> Option<Account> {
        self.client.get_account(*address).await.unwrap()
    }

    /// Get an account base state
    ///
    /// # Parameters
    /// * `address` - Pubkey of the account
    ///
    /// # Panics
    /// If the account is not a valid ATA (so does not own tokens)
    pub async fn get_account_state(&mut self, address: &Pubkey) -> state::Account {
        let res = self.get_account(address).await.unwrap();
        StateWithExtensions::<state::Account>::unpack(&res.data)
            .unwrap()
            .base
    }

    /// Get the amount of tokens owned by the account.
    ///
    /// If the account doesn't exist, `None` will be returned.
    ///
    /// # Parameters
    /// * `address` - Address of the account for which to get the amount of tokens
    ///
    /// # Panics
    /// If the account is not a valid ATA (so does not own tokens)
    pub async fn get_token_amount(&mut self, address: &Pubkey) -> Option<u64> {
        let res = self.get_account(address).await?;
        let state = StateWithExtensions::<state::Account>::unpack(&res.data)
            .unwrap()
            .base;
        Some(state.amount)
    }

    /// Get a mint base state
    ///
    /// # Parameters
    /// * `address` - Pubkey of the mint
    ///
    /// # Panics
    /// If the account is not a valid Mint.
    pub async fn get_mint_state(&mut self, address: &Pubkey) -> Mint {
        let res = self.get_account(address).await.unwrap();
        StateWithExtensions::<state::Mint>::unpack(&res.data)
            .unwrap()
            .base
    }

    /// Get the extension state for a mint.
    ///
    /// # Parameters
    /// * `address` - Pubkey of the mint
    ///
    /// # Errors
    /// If the account is not a valid Mint
    pub async fn get_mint_state_with_extensions<T: Extension + Pod>(
        &mut self,
        address: &Pubkey,
    ) -> Option<T> {
        let res = self.get_account(address).await?;
        let state = StateWithExtensions::<state::Mint>::unpack(&res.data).ok()?;
        state.get_extension::<T>().cloned().ok()
    }

    /// Get the Metadata from a Mint.
    ///
    /// # Parameters
    /// * `address` - Pubkey of the mint
    ///
    /// # Panics
    /// If the account is not a valid Mint
    pub async fn get_mint_metadata(&mut self, address: &Pubkey) -> Option<TokenMetadata> {
        const META_START: usize = 238;
        let metadata = self
            .get_mint_state_with_extensions::<MetadataPointer>(address)
            .await?;
        let key: Option<Pubkey> = metadata.metadata_address.into();
        assert_eq!(
            key.unwrap(),
            *address,
            "Metadata was not present on the mint"
        );
        let data = self.get_account(&key.unwrap()).await.unwrap();
        TokenMetadata::try_from_slice(&data.data[META_START..]).ok()
    }

    /// Loads a PDA data from an account.
    ///
    /// # Parameters
    /// * `account` - Account from which to read the data
    ///
    /// # Errors
    /// If the given account does not contain the expected data.
    pub async fn from_account<T>(&mut self, account: &Pubkey) -> Option<T>
    where
        T: BorshDeserialize + BangkPda + Debug,
    {
        let data = self.get_account(account).await?.data;
        // println!("Got data: {data:?}");
        let res = T::try_from_slice(&data).ok()?;
        // println!("Loaded account: {res:#?}");
        if !res.is_valid() {
            return None::<T>;
        }
        Some(res)
    }

    /// Adds a new wallet to the testing environment
    ///
    /// # Parameters
    /// * `client` - Name of the client to add.
    ///
    /// # Returns
    /// * Pubkey of the client's wallet.
    ///
    /// # Panics
    /// If the keypair couldn't be generated
    #[must_use]
    pub async fn add_wallet(&mut self, name: &str) -> Pubkey {
        println!("adding wallet for user '{name}'");
        let keypair = keypair_from_seed_phrase_and_passphrase(name, "passphrase").unwrap();
        let key = keypair.pubkey();
        self.wallets.insert(name.into(), keypair);

        let Some(api_key) = self.wallets.get("API") else {
            panic!("no API key in the environment");
        };
        let instruction = transfer(&api_key.pubkey(), &key, 1_000_000_000);
        assert!(
            self.execute_transaction(&[instruction], &["API"])
                .await
                .is_ok(),
            "could not fund the wallet for {name}"
        );

        key
    }
}
