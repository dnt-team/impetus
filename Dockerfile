# This is the first stage: the build environment
FROM rustlang/rust:nightly as builder

# Install dependencies
RUN apt-get update -y && \
    apt-get install -y build-essential git clang curl libssl-dev llvm libudev-dev make protobuf-compiler

# Install Rust wasm target for Substrate wasm engine
RUN rustup target add wasm32-unknown-unknown

WORKDIR /impetus

# Copy the project files
COPY . .

# Build the binary
RUN cargo build --locked --release

# This is the second stage: the final image
FROM ubuntu:focal

# Set the working directory and copy the binary from the previous stage
WORKDIR /impetus
COPY --from=builder /impetus/target/release/impetus /usr/local/bin

# Create a new user for the container
RUN useradd -m -u 1000 -U -s /bin/sh -d /impetus impetus && \
    mkdir -p /data /impetus/.local/share && \
    chown -R impetus:impetus /data && \
    ln -s /data /impetus/.local/share/impetus && \
    rm -rf /usr/bin /usr/sbin

# Set the user and expose the ports
USER impetus
EXPOSE 9944 9933 9615
VOLUME ["/data"]

# Set the command to run the binary
ENTRYPOINT ["/usr/local/bin/impetus"]
