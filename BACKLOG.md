# Praetorium active backlog

One item at a time. This is a compact public working queue, not a replacement for historical research or an exhaustive feature specification. A listed behavior is a review target until reproduced, not a confirmed bug in the current source.

## Next

- [x] **DEV-01 — Local alpha build and initial dashboard acceptance:** owner reports targeted changelog regression passed; locked Rust 1.100.0 Windows release build at `8dd4efb` succeeded in 8m 35s using a short persistent Cargo target (three nonblocking manifest warnings). Owner screenshot confirms Praetorium 0.1.0-alpha.1, commander/ship/location data and offline isolated dashboard. Live journal updates, explicit profile-separation acceptance and effective Codex configuration remain unverified; no full workspace verification or release approval implied.
- [ ] **DEV-02 — Measure workflow during normal feature development:** record actual same-account quota before/after a fresh focused assignment and compare local usage counters when available. Do not commission a separate AI assignment solely to measure usage. Keep raw session records private.

## Reliability queue

- [ ] **REL-01 — Market and finances:** verify zero vs unknown credits/stock, per-field source precedence, market freshness, trade eligibility and observed vs estimated carrier data. Split into individual changes.
  - [ ] **REL-01A — Commander credits:** verify whether an explicitly reported zero-credit balance is incorrectly ignored or retains a stale nonzero value. If reproduced, fix only that distinction and add focused regression coverage; absent/unknown balances must remain distinct from observed zero.
- [ ] **REL-02 — Colonization and exobiology:** verify remote depot scope, attribution/accounting, biological eligibility and mapping/predictor data correctness.
- [ ] **REL-03 — Limits and workers:** review compressed-import and fitting-payload bounds, terminal worker failures, scoped input and clipboard preference behavior.
- [ ] **REL-04 — Performance and UX:** profile observed bottlenecks, check responsiveness, accessibility and map usability; avoid speculative refactoring.

## Product queue

- [ ] **AI-01 — Native OpenAI:** design and implement the smallest useful read-only conversation slice under independent provider authorization, then expand capabilities incrementally.
- [ ] **INT-01 — Connected gameplay intelligence:** trading, fleet logistics, navigation, exploration, exobiology and supported community integrations.
- [ ] **INT-02 — Objectives and memory:** explicit user-controlled goals and contextual planning.
- [ ] **INT-03 — Voice and command interfaces:** VoiceAttack interoperability, desktop controls, tablet/secondary display and VR.

Keep individual regression findings, implementation evidence and upstream applicability in the [improvement register](docs/GEN2_IMPROVEMENTS.md). Do not copy private research or local logs into this public backlog. Outstanding release acceptance is not silently waived; see the register. Retire obsolete branch references only when their commits are included and the local worktree is safe.
