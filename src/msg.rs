use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128};
use crate::state::{OrderBy, AuctionInfo, TokenAuctionState, Bid};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ReceiveNft(cw721::Cw721ReceiveMsg),
    PlaceBid {
        token_id: String,
        token_address: String,
    },
    CancelAuction {
        token_id: String,
        token_address: String,
    },
    Claim {
        token_id: String,
        token_address: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AuctionInfo)]
    AuctionInfos {
        token_address: Option<String>,
        start_after: Option<String>,
        limit: Option<u64>,
    },
    #[returns(TokenAuctionState)]
    AuctionState { auction_id: Uint128 },
    #[returns(Vec<Bid>)]
    Bids {
        auction_id: Uint128,
        start_after: Option<u64>,
        limit: Option<u64>,
        order_by: Option<OrderBy>,
    },
}


#[cw_serde]
pub enum Cw721CustomMsg {
    StartAuction {
        start_time: u64,
        duration: u64,
        coin_denom: String,
        min_bid: Option<Uint128>,
    },
}