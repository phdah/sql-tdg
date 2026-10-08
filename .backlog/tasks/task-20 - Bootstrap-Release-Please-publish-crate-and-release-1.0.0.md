---
id: TASK-20
title: Bootstrap Release Please publish crate and release 1.0.0
status: Done
assignee: []
created_date: '2026-10-04'
updated_date: '2026-10-08'
labels: []
milestone: m-2
dependencies:
  - TASK-19
  - TASK-21
  - TASK-23
---

## Description

Complete milestone m-2 by establishing automated Rust releases with Release Please and publishing
the finished sql-tdg CLI as the first stable `1.0.0` release.

The crate, CLI binary, Git tag, GitHub release, changelog, and crates.io package must share one
version. Conventional Commits are the release-classification input. The bootstrap release is
`1.0.0`; subsequent releases follow SemVer.

The release workflow must follow the same release model already proven in
`phdah/sql-semantic-protocol`: Release Please owns version/changelog/release PR creation, and the
release path verifies the crate before publishing it to crates.io.

## Progress

Release Please PR #38 was merged with the maintainer's TASK-21 sign-off included.
It created Git tag and public GitHub release `v1.0.0` at commit
`e7f03c7df2dcda7855d047531b45d15f9852847a`. The completed
[release workflow](https://github.com/phdah/sql-tdg/actions/runs/37835672502)
passed released-version verification and `cargo publish --locked` to crates.io.
The [release-candidate workflow](https://github.com/phdah/sql-tdg/actions/runs/37832216268)
passed Rust checks, dbt Core E2E, `cargo publish --dry-run --locked`, and installation
of the packaged CLI with version and `--help` assertions. The one-time `release-as`
bootstrap override is removed in the release-finalization PR.

Registry installation with `cargo install sql-tdg --version 1.0.0` can be used
for an independent consumer smoke test; the workflow directly verifies package
publication and the previously packaged CLI installation.

## Acceptance Criteria

- [x] Release Please is configured for the Rust crate and creates release PRs from Conventional Commits.
- [x] The initial Release Please bootstrap targets version `1.0.0`.
- [x] `Cargo.toml` contains the package metadata required for a public crates.io release.
- [x] The sql-tdg CLI is installable from the packaged crate and the installed binary exposes the documented command surface.
- [x] Release Please keeps the Cargo package version, changelog, Git tag, and GitHub release aligned.
- [x] The release workflow performs `cargo publish --dry-run` or an equivalent package verification before publication.
- [x] A successful Release Please release publishes the exact released crate to crates.io using `CARGO_REGISTRY_TOKEN`.
- [x] crates.io publication occurs only for a real Release Please release and is safe against duplicate publication attempts.
- [x] CI/release checks fail if the package cannot be built, tested, packaged, or installed as the released CLI.
- [x] The `v1.0.0` GitHub release is created from the merged Release Please PR.
- [x] Version `1.0.0` is available from crates.io and can be installed with Cargo.
- [x] Any one-time Release Please bootstrap override is removed after the initial `1.0.0` release.
- [x] Milestone m-2 is closed only after all preceding tasks are complete and both the GitHub `v1.0.0` release and crates.io package exist.

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Shipped the first stable version 1.0.0: Release Please PR #38 merged, the
`v1.0.0` GitHub release published, and the crates.io package published by the
successful tagged-release workflow. Recorded maintainer dbt sign-off in TASK-21,
verified the release-candidate checks and published CLI packaging, and removed
the bootstrap-only Release Please version override. This completes m-2.
<!-- SECTION:FINAL_SUMMARY:END -->
