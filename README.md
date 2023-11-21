# Multipars

This software implements the Multipars Beaver triple generation protocol in Rust.
It was tested using Rust version `rustc 1.76.0-nightly (9a66e4471 2023-11-19)`.

## Development Environment

To compile Multipars, you need a development environment with dependencies such as the Rust compiler.
We have a `shell.nix` that describes these dependencies.
When using the Nix package manager, you can enter the development environment simply by running:

```
nix-shell
```

We tested this using the `nixos-23.05` channel.

## Execution

In this section we describe how to run (for performance benchmarks) our LowGear-type triple generation protocol in the two-party setup.
To compile it, run

```
cargo build --release --example low_gear
```

Then run the compiled binary to see available command line options:

```
target/release/examples/low_gear --help
```

For instance, the following command runs the protocol for k=s=32 with both parties in a single process and 4 threads per party:

```
target/release/examples/low_gear --player both --threads 4 --batches 4 -k32 -s32
```

The parameters (k, s) must be one of (32, 32), (64, 64), or (128, 64).

In order to run players in different processes (which could run on different machines), use `--player zero` for party 0 and `--player one` for party 1.
In this case, you also need to configure network addresses and UDP ports.
Example:

```
# Party 0's terminal/machine:
target/release/examples/low_gear --p0-addr 127.0.0.1:5000  --p1-addr 127.0.0.1:5001 --player zero

# Party 1's terminal/machine:
target/release/examples/low_gear --p0-addr 127.0.0.1:5000  --p1-addr 127.0.0.1:5001 --player one
```

In case the parties run on different machines, you need to replace `127.0.0.1` by the respective IP addresses.

## Logging

You can set the environment variable `RUST_LOG=info` or `RUST_LOG=debug` at runtime to enable more verbose logging.
