#!/bin/sh

run() {
    RUSTC=$1

    cargo +$RUSTC test
    sh -c "cd custom-format-macros && cargo +$RUSTC test"
    sh -c "cd custom-format-tests  && cargo +$RUSTC test"
}

run 1.54
run stable
run nightly
