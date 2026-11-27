# Praetorium active backlog

One requirement or coherent slice at a time. Original G2 identifiers and phase numbering are retained; REL/AI/INT identifiers are aliases, not replacements. Findings below originated in the earlier audit; verify the current source before changing it. This is a concise public work queue, not the complete historical research record.

## Next

- [x] **DEV-01 — Local alpha build and initial dashboard acceptance:** owner reports targeted changelog regression passed; locked Rust 1.100.0 Windows release build at `8dd4efb` succeeded in 8m 35s using a short persistent Cargo target (three nonblocking manifest warnings). Owner screenshot confirms Praetorium 0.1.0-alpha.1, commander/ship/location data and offline isolated dashboard. Live journal updates, explicit profile-separation acceptance and effective Codex configuration remain unverified; no full workspace verification or release approval implied.
- [ ] **DEV-02 — Measure workflow during normal feature development:** record actual same-account quota before/after a fresh focused assignment and compare local usage counters when available. Do not commission a separate AI assignment solely to measure usage. Keep raw session records private.

## 01D-A — Storage and protected operation safety

- [x] G2-023 / G2-029 / G2-031 — Durable storage, protected operations and scoped upload retirement. Integrated as ENX-023–025. Earlier verification is recorded in the improvement register; final production and service authorization remain separate.

## 01D-B — Trading, market and fleet carriers (REL-01)

- [x] **ENX-030 — Carrier transfer balance reconciliation.** Integrated from PR #6; reported zero is authoritative, missing balance retains transfer arithmetic. Focused red/green regression reported on Rust 1.100.0. This is separate from G2-004.
- [ ] **G2-004 (REL-01A) — Known zero-credit trade budget.** Next assignment. The original finding concerns the route-search affordability constraint discarding an explicit zero budget, not just a stale displayed commander balance. Preserve the difference between unknown funds, observed zero and an explicit hypothetical budget. Verify the shared Market/Ask route behavior with focused fixtures for unknown, zero, exactly affordable and higher budgets. A commander balance correction alone does not close this item.
- [ ] **G2-022 — Market precedence.** Keep per-field source/age semantics; resolve same-time conflicting observations deterministically and preserve unknown/incomplete data. Avoid location/access rollback from independent clocks.
- [ ] **G2-024 — Market freshness, deals, alerts and access.** Refresh quotes, include relevant origin/payload inputs in request identity, scope alert generations, and distinguish open, restricted and unknown carrier access across desktop and assistant tools.
- [ ] **G2-025 — Carrier movement authority.** Separate jump requests, estimates and verified movements by carrier identity. Do not infer authoritative route advancement or duplicate history from unrelated or repeated observations.
- [ ] **G2-033 (market/carrier scope) — Worker outcomes.** Handle terminal failures and disconnected workers; retire superseded results and permit retry. Leave unrelated workers for 01D-D.

## 01D-C — Exploration, exobiology, colonisation and statistics (REL-02)

- [ ] **G2-026 — Remote depot scope.** Tie supplier searches to the selected construction depot and its verified location; suppress old results when the selection changes. Do not substitute the commander's location for an unknown remote depot.
- [ ] **G2-027 — Accounting and attribution.** Correct chronological claim/contribution, mining, period and ship attribution; include relevant on-foot activity without borrowing unrelated context.
- [ ] **G2-028 — Exploration and biological accuracy.** Keep missing eligibility data explicit; correct mapping/sale state and stale request labels; evaluate predictions chronologically without future-data leakage.

## 01D-D — Build imports, fitting and remaining interface defects (REL-03)

- [ ] **G2-020 — Bounded ship-build imports.** Limit compressed and decoded inputs with clear errors while retaining supported EDSY/Coriolis/SLEF formats.
- [ ] **G2-032 — Fitting and payload.** Use the same selected mass for thruster support warnings and travel performance; verify the boundary cases.
- [ ] **G2-033 (remaining scope) — Worker outcomes.** Extend terminal failure, cancellation, retry and input-generation handling to the remaining affected desktop workers.
- [x] **G2-035 — Sidebar identity.** Praetorium branding visible in the owner Windows dashboard screenshot; attribution retained.
- [ ] **G2-036 — Isolated clipboard UI.** Make the unavailable next-system clipboard action explicit without overwriting the ordinary saved preference.

## 01D-W — Comprehensive gameplay regression testing

- [ ] Exercise representative affected Windows gameplay journeys after the correction tranches: commander and journal updates, Market/Ask, navigation/carrier, exploration/exobiology, colonisation, fleet/fitting and interface recovery. Preserve isolated profiles, safe copies and existing service gates. Record the tested revision, observed results and outstanding limitations. Reuse valid prior evidence; do not repeat unrelated automated suites by default. Broader verification requires a concrete reason and owner authorization.

## 02 — Performance and existing feature quality (REL-04)

- [ ] **G2-007 — Measured performance.** Compare startup, replay, database/query latency, frame responsiveness and resource use; target observed bottlenecks.
- [ ] **G2-008 — Gameplay usability.** Consistent source, age, unknown, estimated, empty and retry states across affected interfaces; preserve existing workflows.
- [ ] **G2-034 — Galaxy map and accessibility.** Align hit targets with rendered markers, retain all three distance axes, and verify semantic controls, keyboard and platform accessibility where affected.
- [ ] Retain measured quality refinements from G2-011, G2-012 and G2-028 when evidence warrants them; advanced specialist engines remain in 04+.

## 03 — Native OpenAI integration (AI-01)

- [ ] **G2-009 — AI resource and tool boundaries.** Bound context, tokens, retries, time and host work; support cancellation, validate arguments and preserve scoped provenance.
- [ ] **G2-010 — First-class OpenAI provider.** Add optional native text/Responses integration with explicit opt-in, model selection, protected credentials, typed tool rounds and scoped history. Preserve existing Local, Custom, Gemini and Claude functionality. Paid live requests require separate approval.

## 04+ — Advanced gameplay intelligence and specialist integrations

- [ ] **G2-011 (INT-01) — Trading and carrier logistics.** Feasible optimization, route constraints, time/credits, fuel and operating economics with qualified market evidence.
- [ ] **G2-012 (INT-01) — Exploration, biology, mining and fitting intelligence.** Improve calculations and recommendations against appropriate rights-cleared evidence and held-out cases.
- [ ] **G2-013 (INT-02) — Persistent objectives and memory.** Local scoped goals, deterministic comparisons, proposal selection, restart continuity and user-controlled retention/reset.
- [ ] **G2-014 (INT-03) — Conversational voice, VoiceAttack and PCVR.** Opt-in speech, interruption, validated controls, offline fallback and actual host/runtime acceptance; no unattended gameplay automation.
- [ ] **G2-015 (INT-03) — Paired visual devices.** Host-owned typed desktop/tablet/VR presentations, secure pairing, revocation and disconnect handling; prioritize glanceable tasks.

Keep original technical evidence in its existing research record. Record substantive implementation and actual checks in the [improvement register](docs/GEN2_IMPROVEMENTS.md). Do not import private research or logs into the public repository. Check licensing and service terms before adopting specialist code, data or integrations. Do not silently waive outstanding acceptance or publication gates.
