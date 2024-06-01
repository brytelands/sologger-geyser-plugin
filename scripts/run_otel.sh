#!/usr/bin/env bash

set -ex

cargo build --features 'enable_otel'

solana-test-validator --geyser-plugin-config ../config/sologger-geyser-plugin-config.json