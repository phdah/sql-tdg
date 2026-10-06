---
id: TASK-20
title: Bootstrap Release Please publish crate and release 1.0.0
status: In Progress
assignee: []
created_date: '2026-10-04'
updated_date: '2026-10-06 08:23'
labels: []
milestone: m-2
dependencies:
  - TASK-19
  - TASK-21
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

Release Please configuration, crates.io package metadata, version-contract tests, release-candidate
verification, CLI installation checks, publication, and tagged-release recovery are implemented.
TASK-20 remains in progress until the generated 1.0.0 release PR is merged, the `v1.0.0` GitHub
release exists, version 1.0.0 is published to crates.io, and the one-time `release-as` bootstrap
override is removed.

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
- [ ] The `v1.0.0` GitHub release is created from the merged Release Please PR.
- [ ] Version `1.0.0` is available from crates.io and can be installed with Cargo.
- [ ] Any one-time Release Please bootstrap override is removed after the initial `1.0.0` release.
- [ ] Milestone m-2 is closed only after all preceding tasks are complete and both the GitHub `v1.0.0` release and crates.io package exist.
