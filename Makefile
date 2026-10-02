.PHONY: tests unit-tests go-tests rust-tests rust-checks parity tidy

tests: go-tests rust-tests

unit-tests: tests

go-tests:
	go test $$(go list ./... | grep -v tests) -v

rust-tests:
	cargo test --all-targets --all-features

rust-checks:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all-targets --all-features

parity:
	go test ./internals/interop -run '^TestInterop_FullQueryGenerator' -count=1
	cargo test --test full_query

tidy:
	go mod tidy
