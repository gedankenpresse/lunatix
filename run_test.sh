#!/bin/bash

set -euo pipefail

# Test that the project builds using the default target

cargo build --release -p init # TODO: remove this once the kernel is configured to load init dynamically
cargo build --release -p kernel_loader # TODO: remove once kernel modules are configured dynamically
cargo build


# Test support crates using host architecture
cargo test --target x86_64-unknown-linux-gnu -p allocators
cargo test --target x86_64-unknown-linux-gnu -p derivation_tree
cargo test --target x86_64-unknown-linux-gnu -p ksync
cargo test --target x86_64-unknown-linux-gnu -p regs

# Test support crates using miri
cargo +nightly miri setup
# TODO: fix # cargo +nightly miri test -p derivation_tree --target x86_64-unknown-linux-gnu
# TODO: fix # cargo +nightly miri test -p allocators --target x86_64-unknown-linux-gnu
# TODO: add other support crates to miri tests
# TODO: use cross-platform caps of miri to test for target architecture
