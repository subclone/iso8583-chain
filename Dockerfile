# Use a base image with Rust and necessary build tools
FROM paritytech/ci-unified as builder

# Set the working directory
WORKDIR /app

# Copy the substrate node code to the container
COPY . .

# Build the substrate node
RUN --mount=type=cache,target=/usr/local/cargo/registry \ 
    cargo build --release --locked --features docker

# Use a smaller base image for the final image
FROM debian:bullseye-slim

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    curl
# Copy the built substrate node binary from the builder stage
COPY --from=builder /app/target/release/iso8583-chain /usr/local/bin/

# Expose the default substrate node port
EXPOSE 30333 9933 9944 9615

# Set the entrypoint command to start the substrate node
ENTRYPOINT ["iso8583-chain"]
