use bangk_macro::pda;
use bangk_onchain_common::{get_timestamp, pda::PdaType};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use bangk_onchain_common::{pda::BangkPda, Result};

use crate::processor::TIMELOCK_DELAY;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TransferFromReserveTimelockArgs {
    pub creation_time: i64,
    pub target: Pubkey,
    pub amount: u64,
}

impl TransferFromReserveTimelockArgs {
    pub fn new<I>(target: I, amount: u64) -> Result<Self>
    where
        I: Into<Pubkey>,
    {
        Ok(Self {
            creation_time: get_timestamp()?,
            target: target.into(),
            amount,
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

#[pda(kind = PdaType::TimelockInstruction, seed = "TransferFromReserve")]
pub struct TransferFromReserveTimelock {
    pub instructions: Vec<TransferFromReserveTimelockArgs>,
}

impl TransferFromReserveTimelock {
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
            instructions: Vec::new(),
        }
    }
}
