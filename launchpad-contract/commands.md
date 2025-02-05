Export the CCD account, concordium testnet IP and PORT as shell ENV

```bash
export ACC=4DNTjjZiSrRN6Pt2d6x4qm24eHgMZc141Bg4L6WUsSDMTrcJoz GRPC_IP=node.testnet.concordium.com GRPC_PORT=20000
```

### Build and Deploy contract

```bash
# Builds the smart contract
cargo concordium build --schema-embed --schema-out schema.bin --out build/launchpad.wasm.v1

# Deploy the contract on concordium testnet
concordium-client module deploy launchpad.wasm.v1 --sender $ACC --name launchpad-contract-module --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT
```

### Contract init

```bash
concordium-client contract init launchpad-contract-module --parameter-json init.json --contract launchpad --sender $ACC --energy 10000 --name launchpad-contract --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT
```

### Contract update

```bash
concordium-client contract update launchpad-contract --entrypoint cancel --parameter-json cancel.json --sender $ACC --energy 10000 --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT
```

### Create launchpad

```bash
concordium-client contract update launchpad-contract --entrypoint create_launchpad --parameter-json launchpad.json --sender $ACC --amount 1.2 --energy 30000 --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT
```

### Vest

```bash
concordium-client contract update launchpad-contract --entrypoint vest --parameter-json vest.json --sender $ACC --amount 20 --energy 10000 --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT
```

### Live pause

```bash
concordium-client contract update launchpad-contract --entrypoint live_pause --parameter-json live_pause.json --sender $ACC  --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT --energy 10000
```

### View

```bash
concordium-client contract invoke launchpad-contract --entrypoint view --grpc-ip $GRPC_IP --grpc-port $GRPC_PORT
```

### Base64 schema

```bash
cargo concordium build --schema-base64-out -
```

ref: 9769143a0cf908d7039732b14a07f1e1ea3477981a45c5bf25177046c7320b2f
:5214 // contract index
 