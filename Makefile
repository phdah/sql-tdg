all: test lint

test:
	python -m pytest tests

lint:
	pylint sql_tdg
