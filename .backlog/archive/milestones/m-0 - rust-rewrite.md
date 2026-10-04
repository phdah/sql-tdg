---
id: m-0
title: "Rust rewrite"
---

## Description

Replace the active Go implementation with a Rust implementation while preserving supported
behavior and strengthening deterministic, typed, race-free execution.

The rewrite is incremental. Go remains the reference implementation until Rust reaches end-to-end
parity and the final cutover removes the active Go module.
