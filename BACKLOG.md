# Praetorium active backlog

One item at a time. This is a compact public working queue, not a replacement for historical research or an exhaustive feature specification. A listed behavior is a review target until reproduced, not a confirmed bug in the current source.

## Next

- [ ] **DEV-01 — Local alpha acceptance:** use one fresh session only if needed; check the targeted changelog regression, build the pinned Windows application once and inspect name/version, isolated journal read and profile separation. Record actual results and quota before/after. Do not run full CI or a broad workspace test.
- [ ] **DEV-02 — Measure workflow:** compare actual fresh-session counters using the local usage command. Review excessive context, model calls and tool-output volume only if the budget was missed. Keep any raw session records private.

## Reliability queue

- [ ] **REL-01 — Market and finances:** verify zero vs unknown credits/stock, per-field source precedence, market freshness, trade eligibility and observed vs estimated carrier data. Split into individual changes.
- [ ] **REL-02 — Colonization and exobiology:** verify remote depot scope, attribution/accounting, biological eligibility and mapping/predictor data correctness.
- [ ] **REL-03 — Limits and workers:** review compressed-import and fitting-payload bounds, terminal worker failures, scoped input and clipboard preference behavior.
- [ ] **REL-04 — Performance and UX:** profile observed bottlenecks, check responsiveness, accessibility and map usability; avoid speculative refactoring.

## Product queue

- [ ] **AI-01 — Native OpenAI:** design and implement the smallest useful read-only conversation slice under independent provider authorization, then expand capabilities incrementally.
- [ ] **INT-01 — Connected gameplay intelligence:** trading, fleet logistics, navigation, exploration, exobiology and supported community integrations.
- [ ] **INT-02 — Objectives and memory:** explicit user-controlled goals and contextual planning.
- [ ] **INT-03 — Voice and command interfaces:** VoiceAttack interoperability, desktop controls, tablet/secondary display and VR.

Keep individual regression findings, implementation evidence and upstream applicability in the [improvement register](docs/GEN2_IMPROVEMENTS.md). Do not copy private research or local logs into this public backlog. Outstanding release acceptance is not silently waived; see the register. Retire obsolete branch references only when their commits are included and the local worktree is safe.
