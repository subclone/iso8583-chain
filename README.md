# ISO-8583 Chain


## Introduction

## Run

Make sure you have the necessary environment for Substrate development. If not, please refer to the [official document](https://docs.substrate.io/install/).

```bash
cargo build --release

./target/release/iso8583-chain --dev --tmp
```

### Enable offchain worker

- Offchain worker runs every 2 seconds by default.
- To enable offchain worker, you need to insert your keys to the keystore. You can use the following command to insert your keys to the keystore.

```bash
./target/release/iso8583-chain key insert --key-type iso8 --suri "news slush supreme milk chapter athlete soap sausage put clutch what kitten" --scheme sr25519
```

Note that the above private key is used for demo purposes, i.e the trusted oracle and payment processor API expect this key to sign requests from offchain worker.
