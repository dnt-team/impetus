# Impetus Blockchain


### Setup

##### Rust
Complete the [basic Rust setup instructions](https://github.com/substrate-developer-hub/substrate-node-template/blob/main/docs/rust-setup.md).
##### Yarn
Complete the [basic Yarn setup instructions](https://classic.yarnpkg.com/lang/en/docs/install/#windows-stable).

### Run

Use Rust's native `cargo` command to build and launch the impetus node:

```sh
yarn blockchain:start
```

### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
yarn blockchain:build
```

### Embedded Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/impetus-node -h
```

## Run

The provided `cargo run` command will launch a temporary node and its state will be discarded after
you terminate the process. After the project has been built, there are other ways to launch the
node.

### Single-Node Development Chain

Start the development chain with detailed logging:

```bash
yarn blockchain:debug
```
