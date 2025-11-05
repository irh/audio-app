check *args:
  cargo check --all-targets --all-features {{args}}

clippy *args:
  cargo clippy --all-targets --all-features --no-deps {{args}} -- -D warnings

fmt:
  cargo fmt --all -- --check

run bin *args:
  cargo run --bin {{bin}} {{args}}

run-release bin *args:
  just run {{bin}} --release {{args}}

test *args:
  cargo test --all-features {{args}}

watch command *args:
  cargo watch -s "just {{command}} {{args}}"
