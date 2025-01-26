#!/usr/bin/env bash

set -e

cargo publish -p metricus
cargo publish -p metricus_macros
cargo publish -p metricus_allocator
cargo publish -p metricus_agent
