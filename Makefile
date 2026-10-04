.PHONY: tests unit-tests rust-tests rust-checks

tests: rust-tests

unit-tests: tests

rust-tests:
	cargo test --all-targets --all-features

rust-checks:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all-targets --all-features
	cargo doc --no-deps
