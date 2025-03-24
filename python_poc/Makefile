all: test lint

test: .venv
	. .venv/bin/activate && python -m pytest tests

lint:
	pylint sql_tdg

.venv:
	uv venv && . .venv/bin/activate && uv pip install .
