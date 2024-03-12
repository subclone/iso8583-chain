# How to run the demo

## Prerequisites

Compile and run the ISO-8583 chain as described in the [README](../README.md).

## Insert keys to the keystore

To enable offchain worker, you need to insert your keys to the keystore. You can use the following command to insert your keys to the keystore.

```bash
./target/release/iso8583-chain key insert --key-type iso8 --suri "news slush supreme milk chapter athlete soap sausage put clutch what kitten" --scheme sr25519
```

Note that the above private key is used for demo purposes, i.e the trusted oracle and payment processor API expect this key to sign requests from offchain worker.

## 