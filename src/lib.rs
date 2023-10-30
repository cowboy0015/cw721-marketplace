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
	use contract::{exec_handle_receive_cw721, exec_place_bid};
    use msg::ExecuteMsg;
	match msg {
		ExecuteMsg::ReceiveNft(msg) => exec_handle_receive_cw721(deps, env, info, msg),
        ExecuteMsg::PlaceBid {
            token_id,
            token_address,
        } => exec_place_bid(deps, env, info, token_id, token_address),
    
	}
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: msg::QueryMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        Addr, Deps, DepsMut, Response, Timestamp, Uint128, attr, coins, coin, to_binary,
        testing::{mock_info, mock_env, mock_dependencies},
    };
    use crate::{
        ExecuteMsg, execute, msg::Cw721CustomMsg, InstantiateMsg, instantiate,
        state::{AuctionInfo, TOKEN_AUCTION_STATE, TokenAuctionState, auction_infos},
        error::ContractError
    };
    use cw721::{Cw721ReceiveMsg, Expiration};
    pub const DUMMY_TOKEN_ADDR: &str = "dummy_token_addr";
    pub const DUMMY_TOKEN_OWNER: &str = "dummy_token_owner";
    pub const DUMMY_UNCLAIMED_TOKEN: &str = "dummy_unclaimed_token";

    fn check_auction_created(deps: Deps, min_bid: Option<Uint128>) {
        assert_eq!(
            TokenAuctionState {
                start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
                end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
                high_bidder_addr: Addr::unchecked(""),
                high_bidder_amount: Uint128::zero(),
                coin_denom: "usd".to_string(),
                auction_id: 1u128.into(),
                owner: DUMMY_TOKEN_OWNER.to_string(),
                token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
                token_address: DUMMY_TOKEN_ADDR.to_owned(),
                is_cancelled: false,
                min_bid,
            },
            TOKEN_AUCTION_STATE.load(deps.storage, 1u128).unwrap()
        );
    }

    fn start_auction(deps: DepsMut, min_bid: Option<Uint128>) {
        let custom_msg = Cw721CustomMsg::StartAuction {
            start_time: 100000,
            duration: 100000,
            coin_denom: "usd".to_string(),
            min_bid,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: DUMMY_TOKEN_OWNER.to_owned(),
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            msg: to_binary(&custom_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
        let _res = execute(deps, env, info, msg).unwrap();
    }

    #[test]
    fn test_exe_start_auction() {
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
            sender: DUMMY_TOKEN_OWNER.to_owned(),
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            msg: to_binary(&custom_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
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

    #[test]
    fn test_exe_place_bid() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            Response::new().add_attributes(vec![
                attr("action", "bid"),
                attr("token_id", DUMMY_UNCLAIMED_TOKEN),
                attr("bider", info.sender),
                attr("amount", "100"),
            ]),
            res
        );

        assert_eq!(
            AuctionInfo {
                auction_ids: vec![Uint128::from(1u128)],
                token_address: DUMMY_TOKEN_ADDR.to_owned(),
                token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            },
            auction_infos()
                .load(
                    &deps.storage,
                    &(DUMMY_UNCLAIMED_TOKEN.to_owned() + DUMMY_TOKEN_ADDR)
                )
                .unwrap()
        );

        // // let mut expected_response = AuctionStateResponse {
        // //     start_time: Expiration::AtTime(Timestamp::from_seconds(100)),
        // //     end_time: Expiration::AtTime(Timestamp::from_seconds(200)),
        // //     high_bidder_addr: "sender".to_string(),
        // //     high_bidder_amount: Uint128::from(100u128),
        // //     auction_id: Uint128::from(1u128),
        // //     coin_denom: "usd".to_string(),
        // //     is_cancelled: false,
        // //     min_bid: None,
        // // };

        // // let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        // // assert_eq!(expected_response, res);

        // env.block.time = Timestamp::from_seconds(160);
        // let info = mock_info("other", &coins(200, "usd".to_string()));
        // let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        // assert_eq!(
        //     Response::new()
        //         .add_message(CosmosMsg::Bank(BankMsg::Send {
        //             to_address: "sender".to_string(),
        //             amount: coins(100, "usd")
        //         }))
        //         .add_attributes(vec![
        //             attr("action", "bid"),
        //             attr("token_id", DUMMY_UNCLAIMED_TOKEN),
        //             attr("bider", info.sender),
        //             attr("amount", "200"),
        //         ]),
        //     res
        // );

        // // expected_response.high_bidder_addr = "other".to_string();
        // // expected_response.high_bidder_amount = Uint128::from(200u128);
        // // let res = query_latest_auction_state_helper(deps.as_ref(), env.clone());
        // // assert_eq!(expected_response, res);

        // env.block.time = Timestamp::from_seconds(170);
        // let info = mock_info("sender", &coins(250, "usd".to_string()));
        // let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // assert_eq!(
        //     Response::new()
        //         .add_message(CosmosMsg::Bank(BankMsg::Send {
        //             to_address: "other".to_string(),
        //             amount: coins(200, "usd")
        //         }))
        //         .add_attributes(vec![
        //             attr("action", "bid"),
        //             attr("token_id", DUMMY_UNCLAIMED_TOKEN),
        //             attr("bider", info.sender),
        //             attr("amount", "250"),
        //         ]),
        //     res
        // );

        // // expected_response.high_bidder_addr = "sender".to_string();
        // // expected_response.high_bidder_amount = Uint128::from(250u128);
        // // let res = query_latest_auction_state_helper(deps.as_ref(), env);
        // // assert_eq!(expected_response, res);
    }

    #[test]
    fn test_exec_place_bid_non_existing_auction() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(DUMMY_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_string(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };
        let info = mock_info("bidder", &coins(100, "usd"));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionDoesNotExist {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_place_bid_auction_not_started() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info(DUMMY_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(50u64);

        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionNotStarted {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_place_bid_ended_auction() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info(DUMMY_TOKEN_OWNER, &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(300);

        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_place_bid_owner_cannot_bid() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(DUMMY_TOKEN_OWNER, &coins(100, "usd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::TokenOwnerCannotBid {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_place_bid_highest_bidder_cannot_bid() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("sender", &coins(200, "usd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(
            ContractError::HighestBidderCannotOutBid {},
            res.unwrap_err()
        );
    }

    #[test]
    fn test_exec_place_bid_smaller_than_highest_bid() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);
        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();

        env.block.time = Timestamp::from_seconds(160);
        let info = mock_info("other", &coins(50, "usd".to_string()));
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::BidSmallerThanHighestBid {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_place_bid_invalid_coins() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);
        check_auction_created(deps.as_ref(), None);

        env.block.time = Timestamp::from_seconds(150);

        let error = ContractError::InvalidFunds {
            msg: "Auctions require exactly one coin to be sent.".to_string(),
        };
        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_string(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        // No coins
        let info = mock_info("sender", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        // Multiple coins
        let info = mock_info("sender", &[coin(100, "usd"), coin(100, "uluna")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        let error = ContractError::InvalidFunds {
            msg: "No usd assets are provided to auction".to_string(),
        };

        // Invalid denom
        let info = mock_info("sender", &[coin(100, "uluna")]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert_eq!(error, res.unwrap_err());

        // Correct denom
        let info = mock_info("sender", &[coin(0, "usd")]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(error, res.unwrap_err());
    }
    #[test]
    fn test_exec_start_auction_start_time_in_past() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721CustomMsg::StartAuction {
            start_time: 100000,
            duration: 100000,
            coin_denom: "usd".to_string(),
            min_bid: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: DUMMY_TOKEN_OWNER.to_owned(),
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            msg: to_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg);

        assert_eq!(
            ContractError::InvalidStartTime {
                current_time: env.block.time.nanos() / 1000000,
                current_block: env.block.height,
            },
            res.unwrap_err()
        );
    }

    #[test]
    fn test_exec_start_auction_zero_start_time() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721CustomMsg::StartAuction {
            start_time: 0,
            duration: 1,
            coin_denom: "usd".to_string(),
            min_bid: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: DUMMY_TOKEN_OWNER.to_owned(),
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            msg: to_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::InvalidExpiration {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_start_auction_zero_duration() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let hook_msg = Cw721CustomMsg::StartAuction {
            start_time: 100,
            duration: 0,
            coin_denom: "usd".to_string(),
            min_bid: None,
        };
        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: DUMMY_TOKEN_OWNER.to_owned(),
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            msg: to_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0);

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
        let res = execute(deps.as_mut(), env, info, msg);

        assert_eq!(ContractError::InvalidExpiration {}, res.unwrap_err());
    }
}