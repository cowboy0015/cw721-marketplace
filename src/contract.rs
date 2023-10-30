use cosmwasm_std::{from_binary, attr, ensure, Uint128, DepsMut, Env, MessageInfo, Storage, Response, BlockInfo, Timestamp, Addr};
use crate::{
    msg::{Cw721CustomMsg},
    state::{BIDS, TOKEN_AUCTION_STATE, NEXT_AUCTION_ID, TokenAuctionState},
    error::{ContractError},
};
use cw721::{Cw721ReceiveMsg, Expiration};



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