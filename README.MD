# Building the Project
To build the project, follow these steps:
1. Create a `.cargo` folder and add a file named `config`. In the `config` file, include the following aliases:
```
[alias]
wasm = "build --release --target wasm32-unknown-unknown"
unit-test = "test --lib"
schema = "run --example schema"
```
2. Build the project using the following command:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
```
This command compiles the project for WebAssembly (Wasm) target.

# Project Overview
## src/msg.rs
This file defines various message types used in the contract, including:
- `InstantiateMsg`
- `QueryMsg`
- `ExecuteMsg`

It also introduces `Cw721CustomMsg`, which is designed for defining messages passed by other contracts.

## src/lib.rs
The `lib.rs` file serves as the entry point for the contract. It plays a stub role, with the actual contract implementation residing in the `src/contract.rs` file.

## src/error.rs
In this file, error types that can be emitted by the smart contract are defined. This provides a structured way to handle errors and exceptions.

## src/mock.rs
This file contains utility functions(only one at the moment) for testing, such as:

- `custom_mock_dependencies`: Used to create mock dependencies that include a `querier` for `CW20` coins. This is crucial for testing the contract's functionality.

## src/state.rs
The `state.rs` file manages the state for marketplace management. It introduces a `auction_infos` multiindex for mapping token IDs and token addresses to auction information. `AuctionInfo` contains `auction_ids` as a vector as tokens can be auctioned multiple times. This file also includes helper functions for reading auctions and bids.

## src/contract.rs
The core contract logic is implemented in this file. It is divided into three main categories:

1. `exec_*` functions: These are responsible for handling `ExecuteMsg` messages and executing contract functions.

2. `query_*` functions: These handle `QueryMsg` messages and facilitate data retrieval from the contract.

3. Helper functions: These are added at the end of the file to simplify the logic of message handlers. They provide common functionality and reusability for various parts of the contract.

The project's structure and organization have been designed to separate concerns, making it easier to maintain, test, and understand the codebase. These decisions aim to create a robust and efficient CosmWasm contract.