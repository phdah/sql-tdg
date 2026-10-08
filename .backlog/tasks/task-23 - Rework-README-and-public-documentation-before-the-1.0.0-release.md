---
id: TASK-23
title: Rework README and public documentation before the 1.0.0 release
status: Done
assignee: []
created_date: '2026-10-08 13:22'
updated_date: '2026-10-08 20:19'
labels: []
milestone: m-2
dependencies:
  - TASK-21
  - TASK-21.2
  - TASK-21.5
  - TASK-21.7
  - TASK-21.8
  - TASK-21.9
  - TASK-22
references:
  - 'https://github.com/matiassingers/awesome-readme'
  - README.md
  - AGENTS.md
  - docs/
  - TASK-20
priority: high
type: docs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Final task before the Release Please 1.0.0 PR is merged (TASK-20). Run it after every other release-gate task, so the documentation describes the shipped behavior.

Maintainer instruction (2026-10-08), to be carried out as written:
> In my repo, phdah/sql-tdg, read the README.md and AGENTS.md and rework the README and other documentation in order to make it follow the practices of a good README in a public repository. Follow https://github.com/matiassingers/awesome-readme. Create a PR with the implementation and link the GitHub PR URL back to me.

Context:
- The current README grew feature by feature during development. It mixes library internals, CLI usage, release mechanics (including the one-time `release-as` bootstrap note), and verification steps.
- AGENTS.md holds contributor and agent conventions; public-facing contributor guidance may need to be derived from it without duplicating it.
- Other documentation lives in `docs/` (`rust-dependencies.md`, `rust-rewrite.md`), `CHANGELOG.md` (owned by Release Please), and `python_poc/` (frozen reference).
- Behavior added by the 2.0 protocol adoption must be documented as shipped: exactness errors, comparison-assumption declaration, catalog-less dbt, CTE support, CASE branch coverage, and dbt data tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README.md and AGENTS.md were read in full, and the reworked README follows the practices collected in matiassingers/awesome-readme (for example a clear one-line description, badges, motivation, quick start, installation, usage examples for library, raw SQL CLI, and dbt, supported and unsupported semantics, contributing, and license)
- [x] #2 Every command and code example in the README runs as written against the release candidate, and documented behavior matches the shipped CLI and library
- [x] #3 Internal, historical, or release-bootstrap material (for example the release-as override note and the Python proof of concept) is removed from the README or moved to an appropriate document, with no duplicated content between README, AGENTS.md, and docs/
- [x] #4 Other documentation under docs/ is reviewed and either updated, linked from the README, or removed if obsolete; CHANGELOG.md is left to Release Please
- [x] #5 make rust-checks passes and cargo doc --no-deps succeeds
- [x] #6 The work is submitted as a GitHub pull request on phdah/sql-tdg and its URL is reported back to the maintainer
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: PR #34 reorganizes README.md with verified CLI and library examples, adds docs/usage.md for advanced behavior, removes obsolete Rust migration notes, and adds integration tests covering the README raw SQL and Rust library examples plus the documented binary-collation command. Release Please owns CHANGELOG.md, which remains unchanged. TASK-21 maintainer sign-off is still outstanding; this documentation PR does not satisfy that separate release gate. The README/library smoke tests, Rust formatting, lint, Rust tests, and documentation build are green on commit 16a650c. The dbt Core E2E workflow also passed on the earlier candidate 2e3172d, which differs in Rust test formatting and task tracking only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Reworked README for an open-source audience following awesome-readme examples: concise purpose, status/license badges, quick install and raw-SQL/dbt/Rust entry points, realistic limitations, contributors, and license. Moved detailed runtime semantics, assumptions, dbt constraints, and metadata behavior into docs/usage.md. Removed docs/rust-rewrite.md and docs/rust-dependencies.md because they describe superseded architecture and dependencies. Smoke tests verify the documented raw-SQL CLI command, Rust API sample, and binary-collation example. CHANGELOG.md remains Release Please-owned. PR #34 contains the implementation. The separate maintainer sign-off for TASK-21 remains required before release.
<!-- SECTION:FINAL_SUMMARY:END -->
