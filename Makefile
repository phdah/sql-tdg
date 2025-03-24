tests: unit-tests

unit-tests:
	go test $$(go list ./... | grep -v tests) -v

tidy:
	go mod tidy

