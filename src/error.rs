use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid expiration")]
    InvalidExpiration {},

    #[error("Invalid Start time. Current time: {current_time}. Current block: {current_block}")]
    InvalidStartTime {
        current_time: u64,
        current_block: u64,
    },

    #[error("Overflow")]
    Overflow {},
}

impl From<OverflowError> for ContractError {
    fn from(_err: OverflowError) -> Self {
        ContractError::Overflow {}
    }
}