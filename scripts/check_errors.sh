#!/usr/bin/env bash

set -ex

cargo build
 RUST_LOG=error solana-test-validator \
 --geyser-plugin-config config/geyser-logstash-plugin-config.json \
 || tail -f test-ledger/validator.log