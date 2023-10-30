pub mod msg;
mod contract;
mod error;
mod state;

use cosmwasm_std::{
    Deps, DepsMut, Env, MessageInfo, Response, entry_point, Uint128
};

use {
	msg::InstantiateMsg,
	error::ContractError,
	state::NEXT_AUCTION_ID,
	msg::{ExecuteMsg}
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<(), ContractError> {
    NEXT_AUCTION_ID.save(deps.storage, &Uint128::from(1u128))?;
	Ok(())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response, ContractError> {
	use contract::{exec_handle_receive_cw721};
    use msg::ExecuteMsg;
	match msg {
		ExecuteMsg::ReceiveNft(msg) => exec_handle_receive_cw721(deps, env, info, msg),
	}
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: msg::QueryMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        Addr, Deps, Response, Timestamp, Uint128, to_binary, attr,
        testing::{mock_info, mock_env, mock_dependencies},
    };
    use crate::{
        ExecuteMsg, execute, msg::Cw721CustomMsg, InstantiateMsg, instantiate,
        state::{TOKEN_AUCTION_STATE, TokenAuctionState},
    };
    use cw721::{Cw721ReceiveMsg, Expiration};
    pub const MOCK_TOKEN_ADDR: &str = "dummy_token_addr";
    pub const MOCK_TOKEN_OWNER: &str = "dummy_token_owner";
    pub const MOCK_UNCLAIMED_TOKEN: &str = "dummy_unclaimed_token";

    fn check_auction_created(deps: Deps, min_bid: Option<Uint128>) {
        assert_eq!(
            TokenAuctionState {
                start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
                end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
                high_bidder_addr: Addr::unchecked(""),
                high_bidder_amount: Uint128::zero(),
                coin_denom: "usd".to_string(),
                auction_id: 1u128.into(),
                owner: MOCK_TOKEN_OWNER.to_string(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
                token_address: MOCK_TOKEN_ADDR.to_owned(),
                is_cancelled: false,
                min_bid,
            },
            TOKEN_AUCTION_STATE.load(deps.storage, 1u128).unwrap()
        );
    }

    #[test]
    fn test_execute_start_auction() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let _res = instantiate(deps.as_mut(), env, info, InstantiateMsg {}).unwrap();

        let custom_msg = Cw721CustomMsg::StartAuction {
            start_time: 100000,
            duration: 100000,
            coin_denom: "usd".to_string(),
            min_bid: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: MOCK_TOKEN_OWNER.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            msg: to_binary(&custom_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(MOCK_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("action", "start_auction"),
                attr("start_time", "expiration time: 100.000000000"),
                attr("end_time", "expiration time: 200.000000000"),
                attr("coin_denom", "usd"),
                attr("auction_id", "1"),
            ]),
        );
        check_auction_created(deps.as_ref(), None);
    }
}