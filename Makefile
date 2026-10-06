.PHONY: tests unit-tests rust-tests rust-checks dbt-e2e

tests: rust-tests

unit-tests: tests

rust-tests:
	cargo test --all-targets --all-features

rust-checks:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all-targets --all-features

dbt-e2e:
	cargo test --test dbt_core_e2e -- --ignored --nocapture
