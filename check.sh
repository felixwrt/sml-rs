#!/bin/bash

cargo_test () {
    echo "---------------------------------------------------------------------------------------------------"
    echo "---------------------------------------------------------------------------------------------------"
    echo "  Running with " "$@"
    echo "---------------------------------------------------------------------------------------------------"
    echo "---------------------------------------------------------------------------------------------------"
    cargo test --quiet $@
}

cargo_test 
cargo_test --no-default-features
cargo_test --no-default-features --features=alloc

echo "---------------------------------------------------------------------------------------------------"
echo "---------------------------------------------------------------------------------------------------"
echo "  Running cargo fmt"
echo "---------------------------------------------------------------------------------------------------"
echo "---------------------------------------------------------------------------------------------------"
cargo fmt --all -- --check

echo "---------------------------------------------------------------------------------------------------"
echo "---------------------------------------------------------------------------------------------------"
echo "  Running clippy"
echo "---------------------------------------------------------------------------------------------------"
echo "---------------------------------------------------------------------------------------------------"
cargo clippy -- -D warnings