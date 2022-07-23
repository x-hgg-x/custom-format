cargo +1.45 test --features=runtime
sh -c "cd custom-format-tests && cargo +1.45 test --features=runtime"
sh -c "cd custom-format-macros && cargo +1.45 test --features=runtime"

cargo +1.51 test --workspace --all-features
cargo +stable test --workspace --all-features
cargo +nightly test --workspace --all-features
