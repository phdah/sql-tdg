---
id: TASK-34
title: Add crates.io version, docs.rs, and main-branch CI badges after first release
status: Done
assignee: []
created_date: '2026-10-08'
updated_date: '2026-10-08'
labels: []
dependencies:
  - TASK-20
references:
  - README.md
  - .github/workflows/rust-checks.yml
  - .github/workflows/release-please.yml
  - 'https://github.com/phdah/sql-tdg/pull/34'
  - 'https://github.com/phdah/sql-semantic-protocol/blob/main/README.md'
  - 'https://crates.io/crates/sql-tdg'
  - 'https://docs.rs/sql-tdg'
priority: high
type: task
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After the first stable `sql-tdg` version (v1.0.0) is actually published to crates.io and its Rust API documentation is available on docs.rs, bring the public README badges in line with sql-semantic-protocol: GitHub CI, published crates.io version, docs.rs and MIT license.

Do not add an unavailable or unverified published-version/docs badge before release. The current README PR #34 shows a "Rust checks" badge backed by `rust-checks.yml`, but that workflow is currently `pull_request` only, whereas sql-semantic-protocol's `ci.yml` runs for both PRs and pushes to `main`. A badge reporting default-branch CI must be backed by an actual CI run on `main`, not by Release Please; crates.io and docs.rs badges come from the package registry and documentation service, respectively.

This task is immediate post-release documentation/CI housekeeping. It is separate from the advanced SQL generation milestone and must not delay TASK-20.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Confirm TASK-20's v1.0.0 GitHub release and the published `sql-tdg` crate are available; verify a successful docs.rs build and correct API documentation URL before advertising published docs.
- [x] #2 Change `.github/workflows/rust-checks.yml` to run on both pull requests targeting `main` and pushes to `main`. Preserve independent fmt, lint, rust-tests, doc and dbt Core E2E job results.
- [x] #3 Replace the README's "Rust checks" badge label with "CI" targeting the actual `rust-checks.yml` default-branch GitHub Actions status (explicitly scoped to `main` if appropriate), and verify the badge reflects a completed `main` run rather than Release Please or PR-only runs.
- [x] #4 Add the live crates.io version badge (`https://img.shields.io/crates/v/sql-tdg.svg?cacheSeconds=300`, linked to `https://crates.io/crates/sql-tdg`) and docs.rs badge (`https://docs.rs/sql-tdg/badge.svg`, linked to `https://docs.rs/sql-tdg`) next to CI and MIT license.
- [x] #5 Update README installation guidance to promote `cargo install sql-tdg` as the normal installation path now that publication has occurred; retain `cargo install --path .` as the checkout/development option.
- [x] #6 Verify all README badge/link destinations, published version, documentation availability, and the GitHub workflow's actual default-branch run; avoid broken or misleading badges.
- [x] #7 Run relevant CI and documentation checks and record their results in this task before marking Done. Submit the work in a GitHub pull request.
<!-- AC:END -->

## Progress

- GitHub release `v1.0.0` was published on 2026-10-08; the successful [tagged release workflow](https://github.com/phdah/sql-tdg/actions/runs/37835672502) reports crates.io publication. This confirms the published release; the docs.rs 1.0.0 API documentation and passing badge were subsequently verified.
- Added version and docs badges, promoted `cargo install sql-tdg`, and made the five independent CI checks run on pushes to `main` as well as pull requests.
- [PR CI run 37837662424](https://github.com/phdah/sql-tdg/actions/runs/37837662424) passed all five independent jobs: fmt, lint, Rust tests, rustdoc, and dbt Core E2E.
- [First post-merge `main` CI run](https://github.com/phdah/sql-tdg/actions/runs/37838555022) passed all five independent checks (fmt, lint, tests, docs, and dbt Core E2E).
- [Published badge and docs verification](https://github.com/phdah/sql-tdg/actions/runs/37839020843) passed on a GitHub-hosted runner: Cargo sparse index lists `sql-tdg` 1.0.0, versioned docs at `https://docs.rs/sql-tdg/1.0.0/sql_tdg/` are available, docs.rs badge says `docs: passing`, the crates.io version badge shows `1.0.0`, and the `main` CI SVG says `passing`.
- The GitHub-hosted verification workflow was temporary and removed from the final change; the established independent CI jobs remain unchanged.

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Completed TASK-34 after sql-tdg 1.0.0 publication. The README now links to live CI, crates.io and docs.rs badges and promotes registry installation. The CI workflow runs five independent jobs on pull requests and pushes to main. The first main push passed, and the registry, published API documentation, and rendered badges were verified from GitHub Actions.
<!-- SECTION:FINAL_SUMMARY:END -->
