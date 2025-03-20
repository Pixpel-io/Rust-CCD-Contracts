These are all the commands listed to build, deploy, initialize and test the auction contract.

Building the Smart contract

```bash
cargo concordium build --out build/nft-acution.wasm.v1
```

Export the wallet address, concorcium testnet IP and port as env variables to later use them in the commands
easily

```bash
export ACC=4sjrpri5Yc7UoCo4467bpoAkqNXdLR8LypCje6pUD4zF7eQS83  GRPC_IP=node.testnet.concordium.com  GRPC_PORT=20000
```

Deploying the contract

```bash
concordium-client module deploy build/nft-auction.wasm.v1 --sender $ACC --name auction --grpc-ip $GRPC_IP --grpc-port GRPC_PORT
```

Contract initialization

```bash
concordium-client contract init auction --sender $ACC --contract nft-acution --name auction-instance --energy 1500 --grpc-port $GRPC_PORT --grpc-ip $GRPC_IP
```

To add the item for auction

```bash
concordium-client contract update auction-instance --sender $ACC --entrypoint addItem  --json-params ./schema-artifcats.add_item.json --energy 1500 --grpc-port $GRPC_PORT --grpc-ip $GRPC_IP
```

To bid on a listed item in the contract

```bash
concordium-client contract update auction-instance --sender $ACC --entrypoint bid --amount <'amount-in-ccd'> --json-params ./schema-artifacts/bid.json --energy 1500 --grpc-port $GRPC_PORT --grpc-ip $GRPC_IP
```

To finalize a contract

```bash
concordium-client contract update auction-instance --sender $ACC --entrypoint finalize --json-params ./schema-artifacts/item_index.json --energy 1500 --grpc-port $GRPC_PORT --grpc-ip $GRPC_IP
```

To view the current state of the contract instance

```bash
concordium-client contract ivoke auction-instance --entrypoint view --energy 1500 --grpc-port $GRPC_PORT --grpc-ip $GRPC_IP
```

To view a specefic item state listed for the auction in contract

```bash
concordium-client contract invoke auction-instance --entrypoint viewItemState --json-params ./schema-artifacts/item_index.json --energy 1500 --grpc-port $GRPC_PORT --grpc-ip $GRPC_IP
```