use cosmwasm_schema::{cw_serde};
use cosmwasm_std::{Uint128};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ReceiveNft(cw721::Cw721ReceiveMsg)
}

#[cw_serde]
pub enum QueryMsg {}


#[cw_serde]
pub enum Cw721CustomMsg {
    StartAuction {
        start_time: u64,
        duration: u64,
        coin_denom: String,
        min_bid: Option<Uint128>,
    },
}