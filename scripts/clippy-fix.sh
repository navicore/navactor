#!/usr/bin/env bash

cargo clippy --fix -- \
-W clippy::pedantic \
-W clippy::nursery \
-W clippy::unwrap_used \
-W clippy::expect_used
