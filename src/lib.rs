pub mod msg;
mod contract;
mod error;
mod state;
#[cfg(test)]
pub mod mock;

use cosmwasm_std::{
    Deps, DepsMut, Env, MessageInfo, Response, Uint128, entry_point, to_binary, Binary,
};

use {
	msg::InstantiateMsg,
	error::ContractError,
	state::NEXT_AUCTION_ID,
	msg::{ExecuteMsg, QueryMsg}
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    NEXT_AUCTION_ID.save(deps.storage, &Uint128::from(1u128))?;
	Ok(Response::new())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response, ContractError> {
	use contract::{exec_handle_receive_cw721, exec_place_bid, exec_cancel, exec_claim};
	match msg {
		ExecuteMsg::ReceiveNft(msg) => exec_handle_receive_cw721(deps, env, info, msg),
        ExecuteMsg::PlaceBid {
            token_id,
            token_address,
        } => exec_place_bid(deps, env, info, token_id, token_address),
        ExecuteMsg::CancelAuction {
            token_id,
            token_address,
        } => exec_cancel(deps, env, info, token_id, token_address),
        ExecuteMsg::Claim {
            token_id,
            token_address,
        } => exec_claim(deps, env, info, token_id, token_address),
	}
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: msg::QueryMsg) -> Result<Binary, ContractError> {
	use contract::{query_auction_infos, query_bids, query_auction_state};
    match msg {
        QueryMsg::AuctionInfos {
            token_address,
            start_after,
            limit,
        } => to_binary(&query_auction_infos(deps, token_address, start_after, limit)?).map_err(|err| err.into()),
        QueryMsg::Bids {
            auction_id,
            start_after,
            limit,
            order_by,
        } => to_binary(&query_bids(deps, auction_id, start_after, limit, order_by)?).map_err(|err| err.into()),
        QueryMsg::AuctionState {
            auction_id
        } => to_binary(&query_auction_state(deps, auction_id)?).map_err(|err| err.into())
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        Addr, BankMsg, CosmosMsg, Deps, DepsMut, Response, Timestamp, Uint128, WasmMsg, attr, coins, coin, to_binary, from_binary,
        testing::{mock_info, mock_env, mock_dependencies},
    };
    use crate::{
        ExecuteMsg, execute, query, msg::Cw721CustomMsg, InstantiateMsg, instantiate, QueryMsg,
        state::{AuctionInfo, TOKEN_AUCTION_STATE, TokenAuctionState, auction_infos},
        error::ContractError,
        mock::{custom_mock_dependencies, DUMMY_TOKEN_ADDR, DUMMY_TOKEN_OWNER, DUMMY_UNCLAIMED_TOKEN},
    };

    use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg, Expiration};

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
    fn test_exec_start_auction() {
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
    fn test_exec_place_bid() {
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

    #[test]
    fn test_exec_cancel_no_bids() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info(DUMMY_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: DUMMY_TOKEN_ADDR.to_owned(),
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: DUMMY_TOKEN_OWNER.to_owned(),
                    token_id: DUMMY_UNCLAIMED_TOKEN.to_owned()
                })
                .unwrap(),
                funds: vec![],
            })),
            res
        );

        assert!(
            TOKEN_AUCTION_STATE
                .load(deps.as_ref().storage, 1u128)
                .unwrap()
                .is_cancelled
        );
    }

    #[test]
    fn test_exec_cancel_with_bids() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("bidder", &coins(100, "usd"));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::CancelAuction {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        let info = mock_info(DUMMY_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: DUMMY_TOKEN_ADDR.to_owned(),
                    msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: DUMMY_TOKEN_OWNER.to_owned(),
                        token_id: DUMMY_UNCLAIMED_TOKEN.to_owned()
                    })
                    .unwrap(),
                    funds: vec![],
                }))
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "bidder".to_string(),
                    amount: coins(100, "usd")
                })),
            res
        );

        assert!(
            TOKEN_AUCTION_STATE
                .load(deps.as_ref().storage, 1u128)
                .unwrap()
                .is_cancelled
        );
    }

    #[test]
    fn test_exec_cancel_not_token_owner() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("anyone", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::Unauthorized {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_cancel_ended_auction() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::CancelAuction {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(300);

        let info = mock_info(DUMMY_TOKEN_OWNER, &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionEnded {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_claim_no_bids() {
        let mut deps = custom_mock_dependencies(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        env.block.time = Timestamp::from_seconds(250);

        let msg = ExecuteMsg::Claim {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: DUMMY_TOKEN_ADDR.to_owned(),
                    msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                        recipient: DUMMY_TOKEN_OWNER.to_owned(),
                        token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
                    })
                    .unwrap(),
                    funds: vec![],
                }))
                .add_attribute("action", "claim")
                .add_attribute("token_id", DUMMY_UNCLAIMED_TOKEN)
                .add_attribute("token_contract", DUMMY_TOKEN_ADDR)
                .add_attribute("recipient", DUMMY_TOKEN_OWNER)
                .add_attribute("winning_bid_amount", Uint128::zero())
                .add_attribute("auction_id", "1"),
            res
        );
    }

    #[test]
    fn test_exec_claim() {
        let mut deps = custom_mock_dependencies(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        env.block.time = Timestamp::from_seconds(250);

        let msg = ExecuteMsg::Claim {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let transfer_nft_msg = Cw721ExecuteMsg::TransferNft {
            recipient: "sender".to_string(),
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
        };
        assert_eq!(
            Response::new()
                .add_message(CosmosMsg::Bank(BankMsg::Send {
                    to_address: DUMMY_TOKEN_OWNER.to_owned(),
                    amount: coins(100, "usd"),
                }))
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: DUMMY_TOKEN_ADDR.to_string(),
                    msg: to_binary(&transfer_nft_msg).unwrap(),
                    funds: vec![],
                }))
                .add_attribute("action", "claim")
                .add_attribute("token_id", DUMMY_UNCLAIMED_TOKEN)
                .add_attribute("token_contract", DUMMY_TOKEN_ADDR)
                .add_attribute("recipient", "sender")
                .add_attribute("winning_bid_amount", Uint128::from(100u128))
                .add_attribute("auction_id", "1"),
            res
        );
    }

    #[test]
    fn test_exec_claim_auction_not_ended() {
        let mut deps = custom_mock_dependencies(&[]);
        let mut env = mock_env();
        let info = mock_info("owner", &[]);
        let msg = InstantiateMsg {};
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        start_auction(deps.as_mut(), None);

        let msg = ExecuteMsg::PlaceBid {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        env.block.time = Timestamp::from_seconds(150);

        let info = mock_info("sender", &coins(100, "usd".to_string()));
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Claim {
            token_id: DUMMY_UNCLAIMED_TOKEN.to_owned(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionNotEnded {}, res.unwrap_err());
    }

    #[test]
    fn test_exec_claim_auction_already_claimed() {
        let mut deps = custom_mock_dependencies(&[]);
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
            token_id: "claimed_token".to_string(),
            msg: to_binary(&hook_msg).unwrap(),
        });
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(0u64);

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Auction is over.
        env.block.time = Timestamp::from_seconds(300);

        let msg = ExecuteMsg::Claim {
            token_id: "claimed_token".to_string(),
            token_address: DUMMY_TOKEN_ADDR.to_string(),
        };

        let info = mock_info("any_user", &[]);
        let res = execute(deps.as_mut(), env, info, msg);
        assert_eq!(ContractError::AuctionAlreadyClaimed {}, res.unwrap_err());
    }

    #[test]
    fn test_query_start_auction() {
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
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();


        let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
            sender: "foo_token_owner".to_owned(),
            token_id: "foo_token".to_owned(),
            msg: to_binary(&custom_msg).unwrap(),
        });

        let info = mock_info(DUMMY_TOKEN_ADDR, &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        check_auction_created(deps.as_ref(), None);

        let query_msg = QueryMsg::AuctionInfos {
            token_address: Some(DUMMY_TOKEN_ADDR.to_string()),
            start_after: Some("e".to_string()),
            limit: Some(10),
        };
        let res:Vec<AuctionInfo> = from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();
        assert_eq!(
            vec! [ AuctionInfo {
                    auction_ids: vec![Uint128::from(2u128)],
                    token_address: DUMMY_TOKEN_ADDR.to_string(),
                    token_id: "foo_token".to_string(),
                }
            ],
            res
        );

        
        let query_msg = QueryMsg::AuctionInfos {
            token_address: Some(DUMMY_TOKEN_ADDR.to_string()),
            start_after: Some("g".to_string()),
            limit: Some(10),
        };
        let res:Vec<AuctionInfo> = from_binary(&query(deps.as_ref(), env.clone(), query_msg).unwrap()).unwrap();
        assert_eq!(
            Vec::<AuctionInfo>::new(),
            res
        );

        let query_msg = QueryMsg::AuctionInfos {
            token_address: None,
            start_after: None,
            limit: Some(10),
        };
        let res:Vec<AuctionInfo> = from_binary(&query(deps.as_ref(), env, query_msg).unwrap()).unwrap();
        assert_eq!(
            vec! [ AuctionInfo {
                    auction_ids: vec![Uint128::from(1u128)],
                    token_address: DUMMY_TOKEN_ADDR.to_string(),
                    token_id: DUMMY_UNCLAIMED_TOKEN.to_string(),
                }, AuctionInfo {
                    auction_ids: vec![Uint128::from(2u128)],
                    token_address: "dummy_token_addr".to_string(),
                    token_id: "foo_token".to_string(),
                }
            ],
            res
        );
    }
}