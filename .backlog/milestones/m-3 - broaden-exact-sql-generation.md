---
id: m-3
title: Broaden exact SQL test-data generation
---

## Description

Expand sql-tdg after its stable CLI release beyond exact scalar domains and inner equality joins, so it can construct deterministic witness datasets for analytic outputs, set operations, richer joins, subquery predicates, computed conditions and DML state effects. Add optional result-cardinality and scenario goals and a dialect conformance matrix.

All generation semantics must come from a released SQL Semantic Protocol contract. Protocol prerequisites are tracked as TASK-58 through TASK-65 in phdah/sql-semantic-protocol, proposed in https://github.com/phdah/sql-semantic-protocol/pull/77. The milestone does not imply a fixed release version or schedule. SQL Semantic Protocol remains the only semantic authority and non-proven cases fail closed.
