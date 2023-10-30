use cosmwasm_std::{from_binary, to_binary, attr, ensure, coins, Addr, BankMsg, BlockInfo, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Storage, Timestamp, Uint128, WasmMsg};
use crate::{
    msg::{Cw721CustomMsg},
    state::{BIDS, TOKEN_AUCTION_STATE, NEXT_AUCTION_ID, TokenAuctionState, Bid, auction_infos},
    error::{ContractError},
};
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg, Expiration};



pub fn exec_handle_receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&msg.msg)? {
        Cw721CustomMsg::StartAuction {
            start_time,
            duration,
            coin_denom,
            min_bid,
        } => exec_start_auction(
            deps,
            env,
            msg.sender,
            msg.token_id,
            info.sender.to_string(),
            start_time,
            duration,
            coin_denom,
            min_bid,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn exec_start_auction(
    deps: DepsMut,
    env: Env,
    sender: String,
    token_id: String,
    token_address: String,
    start_time: u64,
    duration: u64,
    coin_denom: String,
    min_bid: Option<Uint128>,
) -> Result<Response, ContractError> {
    ensure!(
        start_time > 0 && duration > 0,
        ContractError::InvalidExpiration {}
    );

    let start_expiration = millisecond_to_expiration(start_time)?;
    let end_expiration = millisecond_to_expiration(start_time + duration)?;

    let block_time = block_to_expiration(&env.block, start_expiration).unwrap();
    ensure!(
        start_expiration.gt(&block_time),
        ContractError::InvalidStartTime {
            current_time: env.block.time.nanos() / 1000000,
            current_block: env.block.height,
        }
    );

    let auction_id = get_and_increment_next_auction_id(deps.storage)?;
    let pk = token_id.to_owned() + &token_address;

    let mut auction_info = auction_infos().load(deps.storage, &pk).unwrap_or_default();
    auction_info.push(auction_id);
    if auction_info.token_address.is_empty() {
        auction_info.token_address = token_address.to_owned();
        auction_info.token_id = token_id.to_owned();
    }
    auction_infos().save(deps.storage, &pk, &auction_info)?;
    
    BIDS.save(deps.storage, auction_id.u128(), &vec![])?;


    TOKEN_AUCTION_STATE.save(
        deps.storage,
        auction_id.u128(),
        &TokenAuctionState {
            start_time: start_expiration,
            end_time: end_expiration,
            high_bidder_addr: Addr::unchecked(""),
            high_bidder_amount: Uint128::zero(),
            coin_denom: coin_denom.clone(),
            auction_id,
            min_bid,
            owner: sender,
            token_id,
            token_address,
            is_cancelled: false,
        },
    )?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "start_auction"),
        attr("start_time", start_expiration.to_string()),
        attr("end_time", end_expiration.to_string()),
        attr("coin_denom", coin_denom),
        attr("auction_id", auction_id.to_string()),
    ]))
}

pub fn exec_place_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let mut token_auction_state = get_token_auction_state(deps.storage, &token_id, &token_address)?;

    ensure!(
        !token_auction_state.is_cancelled,
        ContractError::AuctionCancelled {}
    );

    ensure!(
        token_auction_state.start_time.is_expired(&env.block),
        ContractError::AuctionNotStarted {}
    );
    ensure!(
        !token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionEnded {}
    );

    ensure!(
        token_auction_state.owner != info.sender,
        ContractError::TokenOwnerCannotBid {}
    );

    ensure!(
        info.funds.len() == 1,
        ContractError::InvalidFunds {
            msg: "Auctions require exactly one coin to be sent.".to_string(),
        }
    );

    ensure!(
        token_auction_state.high_bidder_addr != info.sender,
        ContractError::HighestBidderCannotOutBid {}
    );

    let coin_denom = token_auction_state.coin_denom.clone();
    let payment: &Coin = &info.funds[0];
    ensure!(
        payment.denom == coin_denom && payment.amount > Uint128::zero(),
        ContractError::InvalidFunds {
            msg: format!("No {} assets are provided to auction", coin_denom),
        }
    );
    ensure!(
        token_auction_state.high_bidder_amount < payment.amount,
        ContractError::BidSmallerThanHighestBid {}
    );

    let mut messages: Vec<CosmosMsg> = vec![];
    // Send back previous bid unless there was no previous bid.
    if token_auction_state.high_bidder_amount > Uint128::zero() {
        let bank_msg = BankMsg::Send {
            to_address: token_auction_state.high_bidder_addr.to_string(),
            amount: coins(
                token_auction_state.high_bidder_amount.u128(),
                token_auction_state.coin_denom.clone(),
            ),
        };
        messages.push(CosmosMsg::Bank(bank_msg));
    }

    token_auction_state.high_bidder_addr = info.sender.clone();
    token_auction_state.high_bidder_amount = payment.amount;

    let key = token_auction_state.auction_id.u128();
    TOKEN_AUCTION_STATE.save(deps.storage, key.clone(), &token_auction_state)?;
    let mut bids_for_auction = BIDS.load(deps.storage, key.clone())?;
    bids_for_auction.push(Bid {
        bidder: info.sender.to_string(),
        amount: payment.amount,
        timestamp: env.block.time,
    });
    BIDS.save(deps.storage, key, &bids_for_auction)?;
    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "bid"),
        attr("token_id", token_id),
        attr("bider", info.sender.to_string()),
        attr("amount", payment.amount.to_string()),
    ]))
}

pub fn exec_cancel(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    token_address: String,
) -> Result<Response, ContractError> {
    let mut token_auction_state = get_token_auction_state(deps.storage, &token_id, &token_address)?;
    ensure!(
        info.sender == token_auction_state.owner,
        ContractError::Unauthorized {}
    );
    ensure!(
        !token_auction_state.end_time.is_expired(&env.block),
        ContractError::AuctionEnded {}
    );
    let mut messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token_auction_state.token_address.clone(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id,
        })?,
        funds: vec![],
    })];

    // Refund highest bid, if it exists.
    if !token_auction_state.high_bidder_amount.is_zero() {
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: token_auction_state.high_bidder_addr.to_string(),
            amount: coins(
                token_auction_state.high_bidder_amount.u128(),
                token_auction_state.coin_denom.clone(),
            ),
        }));
    }

    token_auction_state.is_cancelled = true;
    TOKEN_AUCTION_STATE.save(
        deps.storage,
        token_auction_state.auction_id.u128(),
        &token_auction_state,
    )?;

    Ok(Response::new().add_messages(messages))
}

fn get_and_increment_next_auction_id(
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    let next_auction_id = NEXT_AUCTION_ID.load(storage)?;

    let incremented_next_auction_id = next_auction_id.checked_add(Uint128::from(1u128))?;
    NEXT_AUCTION_ID.save(storage, &incremented_next_auction_id)?;

    Ok(next_auction_id)
}

fn millisecond_to_expiration(time:u64) -> Result<Expiration, ContractError> {
    ensure!(
        time <= u64::MAX / 1000000,
        ContractError::InvalidExpiration {}
    );

    Ok(Expiration::AtTime(Timestamp::from_nanos(
        time * 1000000,
    )))
}

fn block_to_expiration(block: &BlockInfo, model: Expiration) -> Option<Expiration> {
    match model {
        Expiration::AtTime(_) => Some(Expiration::AtTime(block.time)),
        Expiration::AtHeight(_) => Some(Expiration::AtHeight(block.height)),
        Expiration::Never {} => None,
    }
}

fn get_token_auction_state(
    storage: &dyn Storage,
    token_id: &str,
    token_address: &str,
) -> Result<TokenAuctionState, ContractError> {
    let key = token_id.to_owned() + token_address;
    let latest_auction_id: Uint128 = match auction_infos().may_load(storage, &key)? {
        None => return Err(ContractError::AuctionDoesNotExist {}),
        Some(auction_info) => *auction_info.latest().unwrap(),
    };
    let token_auction_state =
        TOKEN_AUCTION_STATE.load(storage, latest_auction_id.u128())?;

    Ok(token_auction_state)
}