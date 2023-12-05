#!/usr/bin/env bash

set -e

bench_k_s() {
    >&2 echo "Benchmarking k=$1, s=$2:"
    bench_k_s_threads "$1" "$2" 1
    bench_k_s_threads "$1" "$2" 2
    bench_k_s_threads "$1" "$2" 4
    bench_k_s_threads "$1" "$2" 8
    bench_k_s_threads "$1" "$2" 16
}

bench_k_s_threads() {
    if [ "$nproc" -lt "$3" ]; then
        >&2 echo "Skipping benchmark with $3 threads, because the system only has $nproc processing units."
        return
    fi
    >&2 echo "Benchmarking with $3 threads ..."
    result="$(target/release/examples/low_gear "${multipars_args[@]}" "-k$1" "-s$2" --threads "$3" --batches "$3")"
    echo "k=$1, s=$2, threads=$3, triples_per_sec=$result"
}

>&2 echo "Compiling low_gear ..."
cargo build --release --example low_gear

multipars_args=("$@")
nproc="$(nproc)"

bench_k_s 64 64
bench_k_s 128 64
