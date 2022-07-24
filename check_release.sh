#!/bin/sh

run() {
    RUSTC=$1
    FEATURES=$2

    cargo +$RUSTC test --no-default-features $FEATURES
    sh -c "cd custom-format-tests  && cargo +$RUSTC test --no-default-features $FEATURES"
    sh -c "cd custom-format-macros && cargo +$RUSTC test --no-default-features $FEATURES"
}

run 1.45 --features=runtime
run 1.45 --features=runtime,better-parsing

run 1.51 --features=compile-time,runtime
run 1.51 --features=compile-time,runtime,better-parsing

run stable --features=compile-time,runtime
run stable --features=compile-time,runtime,better-parsing

run nightly --features=compile-time,runtime
run nightly --features=compile-time,runtime,better-parsing
