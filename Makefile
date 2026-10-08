.PHONY: tests unit-tests rust-tests rust-checks fmt lint doc dbt-e2e

tests: rust-tests

unit-tests: tests

rust-tests:
	cargo test --all-targets --all-features

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --no-deps

rust-checks: fmt lint rust-tests

dbt-e2e:
	cargo test --test dbt_core_e2e -- --ignored --nocapture
