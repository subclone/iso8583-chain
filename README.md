# ISO-8583 Chain

This is a Substrate-based blockchain that implements the ISO-8583 standard for financial transactions. It is a proof of concept that demonstrates a way to implement a custom runtime module that can be used to build a blockchain that integrates with an existing financial system that uses the ISO-8583 standard.

An integral part of this PoC is the PCIDSS compliant trusted oracle and payment processor server located at [payment-processor](https://github.com/subclone/payment-processor).

## Run

Make sure you have the necessary environment for Substrate development. If not, please refer to the [official document](https://docs.substrate.io/install/).

```bash
cargo build --release

./target/release/iso8583-chain --dev --tmp -loffchain-worker
```

### Offchain Worker

First and foremost, insert the offchain worker key by running this command:

```bash
curl -H "Content-Type: application/json" \
 --data '{ "jsonrpc":"2.0", "method":"author_insertKey", "params":["'"iso8"'", "'"news slush supreme milk chapter athlete soap sausage put clutch what kitten"'", "'"0xd2bf4b844dfefd6772a8843e669f943408966a977e3ae2af1dd78e0f55f4df67"'"],"id":1 }' \
"http://localhost:9944"
```

Note that the above private key is used for demo purposes, i.e the trusted oracle and payment processor API expect this key to sign requests from offchain worker.

## Tests, clippy, fmt and coverage

```bash
# Run check
cargo check --features runtime-benchmarks
# Run all tests: unit tests, integration tests, and doc tests
cargo test --workspace --all-features
# Run clippy
cargo clippy --workspace --all-targets --all-features
# Run fmt
cargo +nightly fmt --all --check
# Run code coverage
cargo tarpaulin --workspace --all-features
```

## Other notes:

This is the high-level overview of components and how they interact:

![iso-8583-overview](https://github.com/subclone/payment-processor/assets/88332432/01c97bed-2ec8-4041-9702-cf079477e9be)


