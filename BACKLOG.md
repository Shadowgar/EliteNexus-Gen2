# Praetorium active backlog

One requirement or coherent slice at a time. Original G2 identifiers and phase numbering are retained; REL/AI/INT identifiers are aliases, not replacements. Findings below originated in the earlier audit; verify the current source before changing it. This is a concise public work queue, not the complete historical research record.

## Next

- [ ] **REC-01 — Restore inherited gameplay functionality (priority).** PR #15 integrated at exact tested head `88372c7` via a documentation-only acceptance commit. Restores the Spansh HTTPS API and configured Ask providers in ordinary mode, retaining isolation and credential validation. Windows release succeeded; 82 focused tests passed, one live test ignored. Owner's preliminary gameplay smoke report on Nov 27: “appears to work.” This is not individual acceptance of every service or planner. Verify normal-mode exobiology, exploration, all route planners, outfitting and material searches, plus user-authorized model requests. No external calls or paid AI tests performed during implementation. Review EDDN, EDSM, Inara, EDAstro and EDGalaxyData separately before restoring service access; keep Inara application approval and independent updater signing requirements. Review restricted historical tools and user-confirmed route actions separately. Fix misleading isolation and effective service status, verify persistent key storage, surface settings write failures, and check functional parity before resuming enhancements. Existing G2 scope and interrupted G2-026/027 work remain intact.

- [x] **DEV-01 — Local alpha build and initial dashboard acceptance:** owner reports targeted changelog regression passed; locked Rust 1.100.0 Windows release build at `8dd4efb` succeeded in 8m 35s using a short persistent Cargo target (three nonblocking manifest warnings). Owner screenshot confirms Praetorium 0.1.0-alpha.1, commander/ship/location data and offline isolated dashboard. Live journal updates, explicit profile-separation acceptance and effective Codex configuration remain unverified; no full workspace verification or release approval implied.
- [ ] **DEV-02 — Measure workflow during normal feature development:** record actual same-account quota before/after a fresh focused assignment and compare local usage counters when available. Do not commission a separate AI assignment solely to measure usage. Keep raw session records private.

## 01D-A — Storage and protected operation safety

- [x] G2-023 / G2-029 / G2-031 — Durable storage, protected operations and scoped upload retirement. Integrated as ENX-023–025. Earlier verification is recorded in the improvement register; final production and service authorization remain separate.

## 01D-B — Trading, market and fleet carriers (REL-01)

- [x] **ENX-030 — Carrier transfer balance reconciliation.** Integrated from PR #6; reported zero is authoritative, missing balance retains transfer arithmetic. Focused red/green regression reported on Rust 1.100.0. This is separate from G2-004.
- [x] **G2-004 (REL-01A / ENX-031) — Known zero-credit trade budget.** Integrated from PR #7. Shared route search enforces zero; unknown balance leaves affordability unchecked; Ask supports an explicit hypothetical budget. Focused galaxy and commander-balance regressions reported passed. Ask test initially failed to compile due to an incorrect write-lock unwrap; corrected in `ee7cb85`, with compilation and execution deferred to 01D-W by owner direction.
- [x] **G2-022 (ENX-032) — Market precedence.** Integrated from PR #8. Independent field clocks and sources, deterministic ties, incomplete report preservation, carrier location/access protection, schema-3 migration and same-time history reconciliation. Reported 27 galaxy tests passed at `22d8dc7`; focused history regression red/green at `a4ab34a`. Legacy history reconstruction is limited to existing points at the replaced timestamp. Production-size migration and live acceptance deferred to 01D-W.
- [x] **G2-024A (ENX-033) — Market quote freshness.** Integrated from PR #9; seven focused galaxy regressions reported passed at `2e1dbc7`. Application compilation and runtime verification deferred to 01D-W.
- [x] **G2-024B (ENX-034) — Deals request identity.** Integrated PR #10; full query and commander context retire superseded results. Application check pending at handoff, test compilation/execution and runtime deferred to 01D-W. Disconnected Deals workers addressed by ENX-038; application acceptance deferred to 01D-W.
- [x] **G2-024C (ENX-035) — Market alert generation consistency.** Integrated PR #11. Eight focused galaxy tests reported passed; desktop compilation, alert regression execution and runtime acceptance deferred to 01D-W.
- [x] **G2-024D (ENX-036) — Carrier access.** Integrated PR #12. Shared reported open/restricted/unknown qualification, conservative public destination filtering and docked-origin exception; nine focused galaxy regressions reported passed. Application compilation and desktop, Ask and MCP verification deferred to 01D-W.
- [x] **G2-024 — Market freshness, deals, alerts and access.** All four implementation slices integrated as ENX-033–036; consolidated acceptance remains in 01D-W.
- [x] **G2-025 (ENX-037) — Carrier movement authority.** Integrated PR #13. Separate scheduled requests, estimated timing and verified movement by carrier identity; routes and history use verified movements. Eleven focused model/journal regressions reported passed. Application compilation and runtime verification deferred to 01D-W.
- [x] **G2-033 (market/carrier scope / ENX-038) — Worker outcomes.** Integrated PR #14. Terminal errors, retries and superseded-result retirement for affected market/carrier workers. Three shared worker tests reported passed in a standalone harness; application compilation, desktop regressions and runtime acceptance deferred to 01D-W. Other consumers remain in 01D-D.

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

- [x] Owner reports Windows release build at `4d4b484`, Rust 1.100.0, `C:/PraetoriumBuild`, 5m 41s, three nonblocking manifest warnings. Initial normal-mode smoke showed EDDN reception and journal state; blocked Spansh Bonus hunt exposed service parity regression. Recovery revision `88372c7` subsequently compiled successfully with 82 focused tests passed and one ignored; owner reported a preliminary gameplay pass. Remaining desktop regressions and consolidated gameplay acceptance are outstanding.

- [ ] G2-033 (market/carrier scope): compile Praetorium and run the Deals and Navigation worker regressions. Verify terminal errors and retry in Market and Fleet Carrier Navigation, and change commander, journal source and selected carrier during requests.

- [ ] G2-025: compile Praetorium and exercise Fleet carrier, Navigation, Dashboard, overlay, Ask and MCP with personal and squadron journal fixtures. Replay a cancellation, reschedule, unrelated location and duplicate arrival. Check that elapsed estimates preserve the last confirmed system and that only a matching carrier's verified movement advances its route, including in isolated mode.

- [ ] G2-024D: compile the application and verify Market quotes, Deals, routes, shopping trips, price alerts, Ask and MCP using synthetic carriers with open, restricted, missing and unrecognized access. Confirm public recommendations require reported open access and that a route can start at the commander's current docked market. Check that changing access during a pending alert retires the earlier result.

- [ ] G2-024C: run `cargo +1.100.0 test --locked -p nexus-app --bin Praetorium price_alert_` using `C:\PraetoriumBuild`. With controlled market updates, modify or remove a watch, jump, and switch commanders while evaluations are pending. Verify that only current observations produce notifications, including speech and overlays, and that overlapping watches produce one notification per commodity and trade direction.

- [ ] G2-024B: complete the pending application check and run `cargo +1.100.0 test --locked -p nexus-app --bin Praetorium deals_` using `C:\PraetoriumBuild`. In Abnormal prices, change system, ship capacity and filters during a scan, refresh while busy, and lose and regain origin for a radius scan. Verify only the current results can be opened or pinned.

- [ ] G2-024A: compile the application tests and exercise Find commodity refresh, quote expiry and unknown-age presentation in the desktop and Ask workflows.

- [ ] G2-022: test schema-3 migration on a disposable production-size schema-2 database copy, preserve rollback backup (older builds reject schema 3), exercise live market/carrier history, and measure added provenance/history storage.

- [ ] Complete the deferred G2-004 Ask regression `shopping_trip_chains_the_stops_from_one_to_the_next` on the consolidated revision; verify the corrected test compiles and runs.

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
