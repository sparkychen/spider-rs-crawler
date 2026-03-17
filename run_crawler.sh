#!/bin/bash

sudo apt install -y xvfb
rm -rf target/ && rm -f Cargo.lock
cargo build
xvfb-run --auto-servernum target/debug/spider-enterprise-crawler --config config/crawler.yaml
# xvfb-run --auto-servernum bazel run //:spider-enterprise-crawler -- --config config/crawler.yaml
