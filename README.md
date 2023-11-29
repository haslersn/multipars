# Multipars

This software implements the Multipars Beaver triple generation protocol in Rust.

## Setup

Compiling Multipars requires the nightly Rust compiler.
Most recently, we tested it using `nightly-2023-11-20`.

### Ubuntu 22.04

On Ubuntu 22.04, Multipars can be set up as follows.
Note that, for the reviewers, we provide a VM where this is already done (see below).

```bash
# Install Rust and choose nightly toolchain:
sudo apt update
sudo apt install -y build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup default nightly-2023-11-20

# Download Multipars and compile low_gear:
git clone -b popets-2024 https://github.com/haslersn/multipars.git
cd multipars
cargo build --release --example low_gear
```

### Artifact Evaluation VM

In HotCRP, the reviewers should have access to our VM (Paper# 13).
On that VM, above setup has already been done, so it suffices to SSH into the VM and then:

```bash
cd multipars
```

## Execution

In this section we describe how to run (for performance benchmarks) our LowGear-type triple generation protocol in the two-party setup.
To compile it, run

```bash
cargo build --release --example low_gear
```

Then run the compiled binary to see available command line options:

```bash
target/release/examples/low_gear --help
```

For instance, the following command runs the protocol for k=s=32 with both parties in a single process and 4 threads per party:

```bash
target/release/examples/low_gear --player both --threads 4 --batches 4 -k32 -s32
```

The parameters (k, s) must be one of (32, 32), (64, 64), or (128, 64).

In order to run players in different processes (which could run on different machines), use `--player zero` for party 0 and `--player one` for party 1.
In this case, you also need to configure network addresses and UDP ports.
Example:

```bash
# Party 0's terminal/machine:
target/release/examples/low_gear --p0-addr 127.0.0.1:5000  --p1-addr 127.0.0.1:5001 --player zero

# Party 1's terminal/machine:
target/release/examples/low_gear --p0-addr 127.0.0.1:5000  --p1-addr 127.0.0.1:5001 --player one
```

In case the parties run on different machines, you need to replace `127.0.0.1` by the respective IP addresses.

## Logging

You can set the environment variable `RUST_LOG=info` or `RUST_LOG=debug` at runtime to enable more verbose logging.
