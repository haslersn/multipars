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

In HotCRP, the reviewers should have access to our two VMs `pets177-base` and `pets281-base`.
On these VMs, above setup has already been done, so it suffices to SSH into each VM and then:

```bash
cd multipars
```

## Artifact Review

Section [Runtime Performance](#runtime-performance) provides instructions on how to reproduce the
numbers for Multipars from Figure 7 in the paper.
In the remaining sections after that, we provide some additional details for interested readers.

## Runtime Performance

Figure 7 in the paper displays the number of triples per second for Multipars and SPDZ2k in the
two-party setup.
In order to reproduce the numbers for Multipars, we provide a script `scripts/benchmark.sh` which
runs Multipars with different parameters and outputs the results to stdout.
The script should be run simultaneously on two different machines, each representing one party.
This requires passing some command line arguments, so that each party knows how to connect to the
other party.
Below we show the required arguments to run on abovementioned VMs `pets177-base` and
`pets281-base`.

We configured the uplink on these VMs to have a limit of 50 Mbit/s and a delay of 50 ms (= 100 ms
RTT between the VMs).
This corresponds to our WAN setup in Figure 7(a).
We don't provide instructions for higher bandwidth scenarios (WAN 500 Mbps or LAN 1 Gbps) as the
performance of Multipars is only marginally better in those scenarios, see Figures 7(b) and 7(c).

Note that these VMs only have 4 vCPUs each, so the script will automatically skip the setups with
8 or 16 threads.

Also note that the artifact evaluation VMs are not as powerful as the VMs used for the evaluation
in our Paper, so the numbers (of triples per second) might not exactly match, though they are very
close: see the example output below.

Here are the required arguments to run the benchmarks on abovementioned VMs:

```bash
# Run on pets177-base, from within the ~/multipars/ folder:
RUST_LOG=info scripts/benchmark.sh --p0-addr [::]:5000  --p1-addr pets281-base:5000 --player zero | tee benchmark-results

# Run on pets281-base, from within the ~/multipars/ folder:
RUST_LOG=info scripts/benchmark.sh --p0-addr pets177-base:5000  --p1-addr [::]:5000 --player one | tee benchmark-results
```

Afterwards, the file `benchmark-results` contains the results.
Here's an example output (`cat benchmark-results`) from our test on the artifact evaluation VMs:

```txt
k=64, s=64, threads=1, triples_per_sec=125.55276038963413
k=64, s=64, threads=2, triples_per_sec=251.890950150742
k=64, s=64, threads=4, triples_per_sec=486.7083242378817
k=128, s=64, threads=1, triples_per_sec=85.75209300194427
k=128, s=64, threads=2, triples_per_sec=176.97391298053313
k=128, s=64, threads=4, triples_per_sec=342.4328305614451
```

## Logging

You can set the environment variable `RUST_LOG=info` or `RUST_LOG=debug` at runtime to enable more verbose logging.

## Manual Execution

In this section we describe how to manually run our LowGear-type triple generation protocol in the
two-party setup.
In case you didn't compile it yet, do so now:

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
In this case, you also need to configure hostnames/addresses and UDP ports.
Example:

```bash
# Party 0's terminal:
target/release/examples/low_gear --p0-addr 127.0.0.1:5000  --p1-addr 127.0.0.1:5001 --player zero

# Party 1's terminal:
target/release/examples/low_gear --p0-addr 127.0.0.1:5000  --p1-addr 127.0.0.1:5001 --player one
```

In case the parties run on different machines, you need to replace `127.0.0.1` by the respective
hostnames/addresses.
The address of the player itself (e.g., `--p0-addr` for player zero) is the listen address for
incoming connections; it should typically be set to the wildcard address `[::]`.
In the following example, you need to replace `$P0_ADDRESS` and `$P1_ADDRESS` with the
hostname/address of the respective parties:

```bash
# Party 0's machine:
target/release/examples/low_gear --p0-addr [::]:5000  --p1-addr $P1_ADDRESS:5001 --player zero

# Party 1's machine:
target/release/examples/low_gear --p0-addr $P0_ADDRESS:5000  --p1-addr [::]:5001 --player one
```
