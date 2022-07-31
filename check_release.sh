#!/bin/sh

run() {
    RUSTC=$1
    shift
    FEATURES="$@"

    cargo +$RUSTC test $FEATURES
    sh -c "cd custom-format-macros && cargo +$RUSTC test"
    sh -c "cd custom-format-tests  && cargo +$RUSTC test $FEATURES"
}

run 1.48 --no-default-features --features=runtime
run 1.51
run stable
run nightly
