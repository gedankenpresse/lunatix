#!/bin/bash

# build init_main.rs so that we have a small binary

rustc -C strip=symbols -C opt-level=2 --target riscv64imac-unknown-none-elf init_main.rs