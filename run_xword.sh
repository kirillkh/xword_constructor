#!/usr/bin/env bash
cargo rustc --release --bin xword  -- -Z orbit -Z unstable-options && time target/release/xword $@
