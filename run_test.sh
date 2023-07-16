#!/bin/bash

set -euo pipefail

cargo build
cargo test --target x86_64-unknown-linux-gnu -p allocators
cargo test --target x86_64-unknown-linux-gnu -p derivation_tree
cargo test --target x86_64-unknown-linux-gnu -p ksync
cargo test --target x86_64-unknown-linux-gnu -p regs
