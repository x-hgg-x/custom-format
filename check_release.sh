#!/bin/sh

run() {
    RUSTC=$1
    FEATURES_1=$2
    FEATURES_2=$3

    cargo +$RUSTC test --no-default-features --features=$FEATURES_1
    cargo +$RUSTC test --no-default-features --features=$FEATURES_1,$FEATURES_2

    sh -c "cd custom-format-tests  && cargo +$RUSTC test --no-default-features --features=$FEATURES_1"
    sh -c "cd custom-format-tests  && cargo +$RUSTC test --no-default-features --features=$FEATURES_1,$FEATURES_2"

    sh -c "cd custom-format-macros && cargo +$RUSTC test --no-default-features"
    sh -c "cd custom-format-macros && cargo +$RUSTC test --no-default-features --features=$FEATURES_2"
}

run 1.45 "runtime" "better-parsing"
run 1.51 "compile-time,runtime" "better-parsing"
run stable "compile-time,runtime" "better-parsing"
run nightly "compile-time,runtime" "better-parsing"
