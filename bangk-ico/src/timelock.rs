// File: bangk-ico/src/timelock.rs
// Project: bangk-onchain
// Creation date: Monday 12 August 2024
// Author: Vincent Berthier <vincent.berthier@bangk.app>
// -----
// Last modified: Wednesday 21 August 2024 @ 19:33:07
// Modified by: Vincent Berthier
// -----
// Copyright © 2024 <Bangk> - All rights reserved

use bangk_macro::pda;
use bangk_onchain_common::{debug, get_timestamp, pda::PdaType, Error};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use bangk_onchain_common::{pda::BangkPda, Result};

use crate::{processor::TIMELOCK_DELAY, UnvestingScheme, WalletType};

/// Data for instructions subjected to time-locks
#[derive(
    Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq, Copy, Clone, Serialize, Deserialize,
)]
pub enum TimelockInstruction {
    /// Transfer from reserve
    TransferFromReserve {
        /// Internal wallet source
        source: WalletType,
        /// Pubkey of the target ATA
        target: Pubkey,
        /// Amount to transfer
        amount: u64,
    },
    PostLaunchInvestment {
        /// Pubkey of the target user
        user: Pubkey,
        /// Custom scheme if any
        scheme: Option<UnvestingScheme>,
        /// Amount to unvest
        amount: u64,
    },
}

/// A time-locked instruction
#[derive(Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Timelock {
    /// Instruction being time-locked
    pub instruction: TimelockInstruction,
    /// Time of creation of the instruction
    pub creation_time: i64,
}

impl Timelock {
    /// Create a new `TimelockInstruction::TransferFromReserve` instruction
    ///
    /// # Parameters
    /// * `target` - The ATA to transfer tokens to,
    /// * `amount` - Number of tokens to transfer.
    pub fn transfer_from_internal_wallet<I>(
        source: WalletType,
        target: I,
        amount: u64,
    ) -> Result<Self>
    where
        I: Into<Pubkey>,
    {
        Ok(Self {
            instruction: TimelockInstruction::TransferFromReserve {
                source,
                target: target.into(),
                amount,
            },
            creation_time: get_timestamp()?,
        })
    }

    /// Create a new `TimelockInstruction::PostLaunchInvestment`
    ///
    ///
    /// # Parameters
    /// * `target` - The user investing,
    /// * `scheme` - The custom unvesting scheme if any,
    /// * `amount` - Number of tokens to transfer.
    pub fn post_launch_investment<I>(
        target: I,
        scheme: Option<UnvestingScheme>,
        amount: u64,
    ) -> Result<Self>
    where
        I: Into<Pubkey>,
    {
        Ok(Self {
            instruction: TimelockInstruction::PostLaunchInvestment {
                user: target.into(),
                scheme,
                amount,
            },
            creation_time: get_timestamp()?,
        })
    }

    /// Checks if the instruction is ready to be executed
    ///
    /// # Errors
    /// If the current timestamp could not be retrieved.
    pub fn is_ready(&self) -> Result<bool> {
        Ok(get_timestamp()? >= self.creation_time.saturating_add(TIMELOCK_DELAY))
    }
}

/// A PDA containing all time-locked instructions.
#[pda(kind = PdaType::TimelockInstruction, seed = "TimelockedInstructions")]
pub struct TimelockPda {
    /// Pending instructions
    pub instructions: Vec<Timelock>,
}

impl<'a> TimelockPda<'a> {
    /// Create a new time locked transfer from reserve instruction.
    ///
    /// * `bump` - Bump of the PDA,
    /// * `target` - Target ATA where the tokens will be transferred,
    /// * `amount` - Amount of tokens to transfer.
    ///
    /// # Errors
    /// If the current timestamp could not be retrieved.
    #[must_use]
    pub const fn new(bump: u8) -> Self {
        Self {
            bump,
            pda_type: Self::PDA_TYPE,
            account: None,
            instructions: Vec::new(),
        }
    }

    fn process_instruction(
        &mut self,
        target: TimelockInstruction,
        payer: &AccountInfo<'a>,
    ) -> ProgramResult {
        let Some(idx) = self
            .instructions
            .iter()
            .position(|instr| target == instr.instruction)
        else {
            return Err(Error::QueuedInstructionNotFound.into());
        };
        debug!("found matching queued operation");

        if !self.instructions[idx].is_ready()? {
            return Err(Error::QueuedInstructionNotReady.into());
        }
        self.instructions.remove(idx);
        self.write(payer)
    }

    /// Checks a transfer from reserve instruction.
    ///
    /// If the instruction exists and is ready, the PDA's state on the blockchain
    /// is updated, otherwise an error is returned.
    ///
    /// # Errors
    /// If the instruction does not exist or if it is not ready.
    pub fn process_transfer_from_internal_wallet(
        &mut self,
        source: WalletType,
        target: &Pubkey,
        amount: u64,
        payer: &AccountInfo<'a>,
    ) -> ProgramResult {
        let instr = TimelockInstruction::TransferFromReserve {
            source,
            target: *target,
            amount,
        };
        self.process_instruction(instr, payer)
    }

    /// Checks a post launch investment instruction
    ///
    /// If the instruction exists and is ready, the PDA's state on the blockchain
    /// is updated, otherwise an error is returned.
    ///
    /// # Errors
    /// If the instruction does not exist or if it is not ready.
    pub fn process_post_launch_investment(
        &mut self,
        user: &Pubkey,
        scheme: Option<UnvestingScheme>,
        amount: u64,
        payer: &AccountInfo<'a>,
    ) -> ProgramResult {
        let instr = TimelockInstruction::PostLaunchInvestment {
            user: *user,
            scheme,
            amount,
        };
        self.process_instruction(instr, payer)
    }
}
