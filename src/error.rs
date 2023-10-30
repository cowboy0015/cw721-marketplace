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

    #[error("AuctionDoesNotExist")]
    AuctionDoesNotExist {},
    
    #[error("AuctionCancelled")]
    AuctionCancelled {},

    #[error("AuctionNotStarted")]
    AuctionNotStarted {},

    #[error("AuctionEnded")]
    AuctionEnded {},

    #[error("TokenOwnerCannotBid")]
    TokenOwnerCannotBid {},

    #[error("InvalidFunds: {msg}")]
    InvalidFunds { msg: String },

    #[error("HighestBidderCannotOutBid")]
    HighestBidderCannotOutBid {},

    #[error("BidSmallerThanHighestBid")]
    BidSmallerThanHighestBid {},
}

impl From<OverflowError> for ContractError {
    fn from(_err: OverflowError) -> Self {
        ContractError::Overflow {}
    }
}