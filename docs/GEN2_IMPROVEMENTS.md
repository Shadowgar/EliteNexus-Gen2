# Gen2 improvement register

EliteNexus-Gen2 builds on [EliteNexus](https://github.com/uruskan/EliteNexus), created by [uruskan](https://github.com/uruskan) with contributions from the EliteNexus community. The journal reader, gameplay model, desktop views, integrations, assistant and local MCP interface described here are inherited systems. Entries credit Gen2 for the specific corrections, extensions and fork decisions they describe. The comparison starts at [the audited upstream commit, `e70f3ddf6b66e5c449412e1e592a9d957f41fedd`](https://github.com/uruskan/EliteNexus/commit/e70f3ddf6b66e5c449412e1e592a9d957f41fedd).

The [changelog](../CHANGELOG.md) describes user-facing changes. The [architecture notes](ARCHITECTURE.md) describe the current design. This register preserves implementation history, evidence and upstream applicability as the design evolves.

## Maintaining the register

Assign a stable `ENX-<number>` identifier to each substantive improvement. Group routine changes when they serve one purpose. Append entries in phase order, preserve previous entries and link to the replacement when a later change supersedes an earlier one. Include the subsystem, classification, upstream baseline and limitation, Gen2 behavior and benefit, source and compatibility notes, regression locations and observed results, commit and PR references, remaining limitations, acceptance status and applicability. Note the scope of independent upstream review and any dependencies on earlier entries. Crate paths are relative to `crates/` and abbreviated filenames are relative to the nearest named source directory.

Use **Proposed**, **Implemented in draft**, **Automated verification passed**, **Runtime verification passed**, **Accepted**, **Merged** and **Released** to describe distinct milestones. Attach the platform, acceptance scope and relevant revision. Record **outstanding limitations** explicitly. A proposed design can have a tracking entry with its blocker, and automated results need a reproducible command and an observed outcome. Keep public entries at the level of the correction, its observable contract and safe regression coverage.

## Verification and acceptance for Phases 01A and 01B

Both phases are **implemented in draft**, have **passed automated Windows verification**, and are **accepted as Windows development milestones**. Both remain draft changes awaiting merge and release. Native GUI, Linux and macOS, live-service and distribution acceptance are outstanding. The executable was not launched for these checks. The recorded runs used synthetic data and local fixtures and left the dependency lockfile unchanged.

The Windows target was `x86_64-pc-windows-msvc`. The three locked commands were `cargo build --locked -p nexus-app`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace`, each with the toolchain selector shown below. The stable toolchain used for these historical runs reported Rust 1.100.0. Floating stable CI remains a separate check.

| Revision | Toolchain selector | Build | Strict Clippy | Workspace tests |
| --- | --- | --- | --- | --- |
| Audited upstream baseline | `+stable` (Rust 1.100.0) | Exit 0 | Exit 0 | Exit 0, 829 passed, 0 failed, 18 ignored |
| [01A initial implementation][a-ask] | `+stable` (Rust 1.100.0) | Exit 0 | Exit 0 | Exit 0, 841 passed, 0 failed, 18 ignored |
| [01A reviewed head][a-head] | `+1.95.0` | Exit 0 | Exit 0 | Exit 0, 843 passed, 0 failed, 18 ignored |
| [01B stacked head][b-head] | `+stable` (Rust 1.100.0) | Exit 0 | Exit 0 | Exit 0, 858 passed, 0 failed, 18 ignored |
| [01B stacked head][b-head] | `+1.95.0` | Exit 0 | Exit 0 | Exit 0, 858 passed, 0 failed, 18 ignored |

Cargo reported inherited unused manifest fields and dependencies and a Windows output-name collision between examples named `live`. The local results include no ignored tests. At the time of this verification, no Gen2 Actions runs or commit checks were observed for either draft head.

## Phase 01A — Model connections, assistant boundaries and minimum Rust

The baseline is the audited upstream commit. Status for each entry is implemented in [draft PR #1][a-pr], automated verification passed on Windows, and accepted as a Windows development milestone. Entry results refer to the Windows verification above unless otherwise stated.

### ENX-001 — Bind model credentials to provider and destination

- **Subsystem and classification.** Ask settings and provider configuration. Security improvement, privacy improvement and user experience improvement.
- **Upstream and Gen2 behavior.** Upstream stored one model key in Ask settings and reused it across connections. Gen2 stores keys per provider and normalized API base, including the path. An ambiguous legacy key stays inactive until the user explicitly associates it with the displayed destination. Users can switch connections and recover the matching saved key without reentering it.
- **Code and compatibility.** `nexus-app/src/settings.rs` and `pages/ask.rs` own the settings and UI. `nexus-ai/src/provider.rs` checks the binding at request time. Gemini and Claude keep their provider endpoints, Local supports keyless use, and compatible Custom endpoints remain available. Settings written before this change can be opened, with explicit action required before using the legacy key. Live provider acceptance remains open.
- **Regression evidence.** The focused settings suite passed 18 tests, including legacy isolation, explicit rebind, and provider/path separation. The provider authentication regression passed in the focused 15-test `nexus-ai` suite. [Implementation][a-ask] · [Reviewed head][a-head]
- **Upstream applicability.** Applicable with minor adaptation to the settings and Ask UI. Review the persisted settings shape and explicit recovery together with the request boundary in ENX-002.

### ENX-002 — Apply one transport and diagnostic policy to AI requests

- **Subsystem and classification.** AI discovery, completions and diagnostics. Security improvement and privacy improvement.
- **Upstream and Gen2 behavior.** Upstream discovery and completion used the app HTTP client and could surface server-provided error text. Gen2 routes both through a dedicated transport that validates the destination, authenticates discovery using the bound provider credential, requires HTTPS for remote addresses, allows cleartext HTTP only at loopback addresses, bypasses ambient proxies for that cleartext loopback connection and refuses redirects. Sensitive headers, provider and destination diagnostics, and settings recovery messages are redacted or narrowed to safe information. This limits accidental disclosure and gives actionable connection categories.
- **Code and compatibility.** `nexus-ai/src/provider.rs`, `anthropic.rs`, `openai.rs`, `agent.rs` and `lib.rs` plus `nexus-app/src/settings.rs` and `assistant.rs`. Existing Claude Messages and OpenAI-compatible adapters remain in use. Custom services need a supported address with no embedded user information, query or fragment, and need a direct endpoint if they previously relied on redirects. This phase adds no OpenAI Responses adapter.
- **Regression evidence.** The focused `nexus-ai` suite passed 15 tests, including endpoint checks and recording loopback discovery covering authentication, redirect refusal and safe error text. A focused settings regression checks recovery diagnostics. [Implementation][a-ask] · [Reviewed head][a-head]
- **Upstream applicability.** The transport and diagnostic changes are applicable with minor adaptation. The destination-bound authentication contract should be reviewed with ENX-001. The local loopback tests can be reviewed without external services.

### ENX-003 — Preserve saved credentials when Windows protection fails

- **Subsystem and classification.** Credential storage and account settings. Security improvement and reliability improvement.
- **Upstream and Gen2 behavior.** Upstream could fall back to unencrypted encoding if Windows DPAPI failed. Gen2 returns a save error and leaves the previous credential in place. Older Windows values remain readable and the UI reports their actual protection. This covers Ask, EDSM and Inara key entry.
- **Code and compatibility.** `nexus-app/src/secrets.rs`, `settings.rs` and `pages/accounts.rs`, with updated fixtures in `sync.rs`. Non-Windows storage retains the inherited encoding, which does not encrypt the credential. This phase makes no automatic migration of older Windows credentials.
- **Regression evidence.** The focused secret-storage suite passed two tests, including synthetic Windows protection failure and the inherited round trip. The settings tests also passed. [Implementation][a-ask] · [Reviewed head][a-head]
- **Upstream applicability.** Directly applicable as a focused change across storage and its save callers. The simulated failure case is independently reviewable, with native GUI verification still open.

### ENX-004 — Retire conversations, discovery and replies when their context changes

- **Subsystem and classification.** Ask lifecycle and model scope. Privacy improvement, reliability improvement and user experience improvement.
- **Upstream and Gen2 behavior.** Upstream conversations and model discovery could outlive changes to the active connection or commander. Gen2 binds conversation and cached discovery to the provider, normalized destination, credential generation, model, commander and journal source. It retires pending work and clears the prior context on a scope change. Stop drops an uncommitted answer and New conversation clears the conversation. Results from a retired worker cannot become the next visible turn.
- **Code and compatibility.** `nexus-app/src/assistant.rs`, `app.rs`, `pages/ask.rs` and `ai_tools.rs`, with commander generation in `nexus-model/src/state.rs`. The active generation prevents an earlier request from becoming valid again when a user switches away and back. Cancellation protects local commits and tool execution as scopes change. Data already sent to the selected model is subject to that provider's policy.
- **Regression evidence.** The focused assistant suite passed three tests covering same-scope turns, Stop, late results, scope changes, discovery invalidation and the tool boundary. The focused AI tool suite passed 66 tests with six ignored, including commander switch-back. [Implementation][a-ask] · [Reviewed head][a-head]
- **Upstream applicability.** The generation and retirement pattern is applicable with minor adaptation. Full connection isolation depends on ENX-001 and ENX-002.

### ENX-005 — Scope queued MCP calls at admission

- **Subsystem and classification.** Local MCP request dispatch. Privacy improvement and reliability improvement.
- **Upstream and Gen2 behavior.** A queued request could wait through a change to the active journal context. Gen2 captures the source epoch and commander scope when admitting a request and checks them when the worker takes the queued call. Requests admitted for a retired source or commander receive a scope error. The inherited opt-in local transport is preserved.
- **Code and compatibility.** `nexus-app/src/mcp.rs`, `mcp/stdio.rs`, `app.rs` and `ai_tools.rs`, using the commander generation from ENX-004. Clients continue to use MCP, with requests across context changes requiring a retry in the new scope. Native GUI and broader platform acceptance remain open.
- **Regression evidence.** Two queue tests assert refusal across source and commander changes. Both are included in the reviewed head's 843 passing workspace tests and the stacked head's 11 passing focused MCP tests. [Source retirement][a-mcp] · [Admission epoch][a-epoch] · [Commander admission][a-head]
- **Upstream applicability.** Applicable with minor adaptation once the commander generation from ENX-004 is available. The queue behavior and tests are independently reviewable.

### ENX-006 — Limit assistant and MCP tools to their authorized scope

- **Subsystem and classification.** Ask and MCP tool catalogue and dispatch. Privacy improvement, security improvement and architecture improvement.
- **Upstream and Gen2 behavior.** Upstream Ask offered tools for raw historical evidence and for applying routes. Gen2 aligns advertised tools with caller authorization, guards commander reads and restricts the exposed set to read-only tools. Local Ask excludes tools that make online lookups, while eligible online lookups retain their existing place for the other callers. Phase 01A withheld `journal_search`, `on_foot`, `credit_history`, `system_factions`, `progress` and route-writing tools. ENX-012 records the later restoration of scoped `journal_search`.
- **Code and compatibility.** `nexus-app/src/ai_tools.rs`, its `bodies.rs`, `buy.rs`, `cmdr.rs` and `engineering.rs` submodules, plus `assistant.rs`, `pages/ask.rs`, `mcp.rs` and `mcp/stdio.rs`. The user guide, Ask capability view, changelog and companion website copy were updated. Local is a caller policy for app tools. Its model endpoint follows ENX-002. The inherited Navigation and Logbook interfaces remain available directly.
- **Regression evidence.** The focused assistant policy test checks restricted historical tools, route writes and Local online lookup availability. It passed with the three assistant tests. The focused AI tool suite passed 66 with six ignored. [Implementation][a-ask] · [Reviewed head][a-head]
- **Upstream applicability.** Applicable with minor adaptation to the tool catalogue and dispatcher. Review caller policy independently and coordinate historical-tool eligibility with ENX-011 and ENX-012. Effectful AI tools need separate authority and verification.

### ENX-007 — Declare the locked dependency minimum and verify it in CI

- **Subsystem and classification.** Rust workspace and verification infrastructure. Compatibility improvement and development infrastructure improvement.
- **Upstream and Gen2 behavior.** The upstream workspace declared Rust 1.93 even though its locked GUI libraries required 1.95. Gen2 declared 1.95.0 and configured CI for locked build, strict workspace Clippy and locked tests for exact minimum and floating stable across Windows, Linux and macOS. The workflow uses read-only repository permissions, checks out without persisted credentials and keys the cache by toolchain. Equivalent guard and boolean cleanups allow strict minimum Clippy to pass without lint exemptions or dependency churn.
- **Code and compatibility.** `Cargo.toml`, `.github/workflows/ci.yml`, `AGENTS.md`, `CONTRIBUTING.md` and existing control-flow sites in `nexus-app`, `nexus-model` and `nexus-uplink`. The development docs cover isolated checks, strict locked commands and recording results. No lockfile change. The Windows results establish the minimum for that target. Native Linux and macOS builds and workflow execution on the fork were still outstanding at these heads.
- **Regression evidence.** At the upstream baseline, minimum-rejection checks with Rust 1.93.0 exited 101 before app compilation. The reviewed 01A head passed locked Windows Rust 1.95.0 build, strict Clippy and all 843 enabled workspace tests. The 01B head repeated the minimum and stable checks with 858 passing tests each. [CI and minimum policy][a-ci] · [Clippy compatibility][a-lint] · [Reviewed head][a-head]
- **Upstream applicability.** Directly applicable and independently reviewable for the same locked dependency graph. Recheck the minimum after a dependency graph change and preserve native platform verification.

## Phase 01B — Journal continuity, ownership and current-state provenance

These entries describe the stacked diff from [the reviewed 01A head][a-head] through [the 01B head][b-head]. Status for each is implemented in [draft PR #2][b-pr], automated verification passed on Windows, and accepted as a Windows development milestone. The locked comparison recorded 858 passes and 18 ignored tests on each Windows toolchain.

### ENX-008 — Continue from replay into live journals, including empty starts and rotation

- **Subsystem and classification.** Journal reader, watcher and core. Bug fix and reliability improvement.
- **Upstream and Gen2 behavior.** Upstream startup and tail selection could skip the first file on an empty start or earlier eligible parts when several appeared during a transition. Gen2 captures the initial file order and size boundaries for replay, then drains later complete records in part order. It carries incomplete bytes across the handoff until the terminating newline appears. This keeps existing and newly appended journal records in order without replaying the same record as a live event.
- **Code and compatibility.** `nexus-journal/src/reader.rs`, `watcher.rs`, `nexus-app/src/core.rs` and the `nexus-model/examples/live.rs` consumer. The source journals are read-only and the cursor is in memory. Recovery cannot reconstruct records absent from the selected source. ENX-009 covers changes to files already seen.
- **Regression evidence.** The focused journal suite passed 30 tests, including empty start with several parts, appends during bulk initialization, and partial records at handoff. [Implementation][b-replay] · [Stacked head][b-head]
- **Upstream applicability.** Applicable with minor adaptation for callers of the watcher API. Review the reader and watcher together, including the updated live example and core consumer.

### ENX-009 — Recover from replaced or truncated journal files

- **Subsystem and classification.** Journal file continuity, recovery and raw index. Bug fix and reliability improvement.
- **Upstream and Gen2 behavior.** The earlier tail and raw index could keep referring to a path after its file changed. Gen2 records open-file identity and complete-record checkpoints, checks continuity during polling, and rebuilds on detected removal, replacement or truncation. Raw rereads verify that the indexed record still matches its file identity and content. Users see a rebuilt projection instead of stale offsets pointing at different records.
- **Code and compatibility.** `nexus-journal/src/watcher.rs`, `reader.rs`, `index.rs`, `Cargo.toml` and `nexus-app/src/core.rs`. File identity uses Windows volume and file ID or Unix device and inode. The derived in-memory index needs no persisted data migration. Exact in-place edits that retain observed metadata and the checked checkpoint samples can evade continuous detection. Recovery depends on the files still present.
- **Regression evidence.** The focused journal suite passed 30 tests including same-size and larger replacement and truncation. The focused core suite passed four tests, including quiet replacement and generation retirement. [Implementation][b-replay] · [Stacked head][b-head]
- **Upstream applicability.** Applicable with minor adaptation. The identity and checkpoint implementation is independently reviewable, with full recovery depending on ENX-008 and the core rebuild behavior in ENX-011.

### ENX-010 — Preserve physical offsets and localized journal text

- **Subsystem and classification.** Journal byte framing and tolerant JSON parsing. Bug fix and compatibility improvement.
- **Upstream and Gen2 behavior.** Upstream's lossy string conversion could shift offsets following invalid bytes, and numeric repair could mishandle surrounding text. Gen2 frames records in physical bytes and skips invalid UTF-8 lines without shifting subsequent offsets. It repairs supported non-finite numeric tokens outside quoted strings while preserving valid localized text. Logbook and scoped search can reopen the intended raw record.
- **Code and compatibility.** `nexus-journal/src/reader.rs`, `watcher.rs`, `raw.rs` and `index.rs`. Valid UTF-8 records keep their original byte locations. Unsupported or incomplete data may still be skipped or deferred. Sanitization does not modify source journals.
- **Regression evidence.** The synthetic invalid-UTF-8 offset regression failed at the Phase 01A baseline, exit 101, and passed in the corrected 30-test journal suite, exit 0. The same suite verifies localized text alongside repaired numeric tokens. [Implementation][b-replay] · [Stacked head][b-head]
- **Upstream applicability.** Directly applicable as focused framing and parser fixes. Independently reviewable with the `reader.rs` and `raw.rs` unit tests, while live parity requires the watcher to use the same byte framing.

### ENX-011 — Bind journal events and replay to source, session and commander

- **Subsystem and classification.** Core replay, model attribution and journal index. Privacy improvement, reliability improvement and architecture improvement.
- **Upstream and Gen2 behavior.** Timeline and active model state could follow a previous commander into later journal parts without sufficient ownership evidence. Gen2 keeps an in-process source generation and distinct session and commander state. Adjacent numbered parts for one session can share context. A gap or unrelated file retires it even when timestamps match. The journal's Frontier ID (FID) establishes the verified commander. Name-only observations stay provisional and can promote compatible state within their session without assigning an owner retroactively to earlier raw records. Historical replay rebuilds local state quietly. Unverified events stay out of live forwarding, and recovery avoids republishing consumed history as live.
- **Code and compatibility.** `nexus-app/src/core.rs`, `nexus-model/src/state.rs`, `nexus-journal/src/index.rs`, `reader.rs` and `watcher.rs`. Event identity uses file generation and physical offset. Arbitrary copied duplicate lines without distinguishing evidence cannot always be put in authoritative order. The model and index are derived in memory.
- **Regression evidence.** The four focused core tests passed, including multipart ownership versus disconnected same-time sessions, provisional promotion and quiet replay across replacement. The standalone model command passed 90 tests. [Replay and ownership][b-replay] · [Disconnected sessions][b-session] · [Stacked head][b-head]
- **Upstream applicability.** Dependent on ENX-008 through ENX-010 for the full implementation. Review session attribution and FID promotion as a unit with the model, index and core fixtures.

### ENX-012 — Provide explicit aggregate Logbook and verified commander journal search

- **Subsystem and classification.** Logbook, Ask and MCP history. Privacy improvement and user experience improvement.
- **Upstream and Gen2 behavior.** Upstream Logbook exposed aggregated raw events and the assistant searched journal files without per-row verified ownership. Gen2 labels the aggregate view **All commanders**, keeps it available even without a current commander when the index has records, and includes records with both known and unknown owners. It restores `journal_search` for Ask, Local Ask and MCP using indexed observations for the active verified FID and current source. Other commanders' and unknown-owner rows are excluded. Search remains bounded and supports narrowing by date.
- **Code and compatibility.** `nexus-app/src/pages/logbook.rs`, `app.rs`, `ai_tools.rs`, `assistant.rs`, `nexus-journal/src/index.rs`, the user guide and changelog. The source and commander's generation must still be current when resolving a tool call. `on_foot`, `credit_history`, `system_factions`, `progress` and effectful AI tools including route writes remain restricted.
- **Regression evidence.** The aggregate Logbook test passed, covering known and unknown indexed rows at their raw offsets. The scoped Ask/Local Ask/MCP history fixture passed in the 55-test focused app AI suite, with four ignored. The focused MCP suite passed 11 tests. [Scoped history][b-replay] · [Aggregate availability][b-log] · [Stacked head][b-head]
- **Upstream applicability.** Dependent on ENX-006, ENX-010 and ENX-011 for scoped AI search. The aggregate view is separately reviewable in `pages/logbook.rs` once the ownership index exists.

### ENX-013 — Qualify companion snapshots before replacing current state

- **Subsystem and classification.** Journal companion-file monitoring and model admission. Bug fix, privacy improvement and reliability improvement.
- **Upstream and Gen2 behavior.** Companion files can remain on disk across commander changes or be observed during a write. Gen2 distinguishes missing, incomplete, unknown, stale, known-empty and current observations. It admits snapshots against the verified active session, a matching journal announcement where applicable, and matching station and system for Market. It refuses older or cleared routes and leaves the last accepted state intact when a file is incomplete. Standalone Status requires current dated evidence from the auto-detected profile and is withheld for explicitly selected sources.
- **Code and compatibility.** `nexus-journal/src/watcher.rs`, `nexus-app/src/core.rs` and `nexus-model/src/state.rs`. Initial companion acquisition stays quiet. A snapshot with insufficient ownership or freshness evidence leaves the view without newly accepted data. GUI acceptance for the companion views remains open.
- **Regression evidence.** The 30-test journal suite passed partial-to-safe recovery. The four-test core suite passed the stale and previous-owner fixture, including unknown standalone Status and known-empty Cargo. [Initial snapshot scope][b-replay] · [Availability and ownership][b-snapshot] · [Stacked head][b-head]
- **Upstream applicability.** Depends on ENX-011 for owner qualification. The partial-file observation and retry behavior is independently reviewable with its watcher test.

### ENX-014 — Keep coordinates tied to the system they describe

- **Subsystem and classification.** Location model and AI lookup origin. Bug fix and reliability improvement.
- **Upstream and Gen2 behavior.** With no coordinates in a new arrival, upstream could retain the previous system's position. Gen2 clears it on a system address change unless new finite coordinates are present. Matching journal observations may retain or restore position. A selected AI origin lookup can recover unknown position from a finite local database record whose system address and name both match. This protects searches and distance calculations from a previous system's origin.
- **Code and compatibility.** `nexus-model/src/location.rs`, `nexus-app/src/ai_tools.rs` and `ai_tools/cmdr.rs`, using existing `nexus-galaxy` lookup interfaces. A missing local match leaves position unknown. Other views still use the existing shared optional model position. Carrier route authority remains separate.
- **Regression evidence.** The focused model provenance test and local AI origin test each passed. They cover matching addresses, distinct destinations and rejection of a mismatched local name. The full application and model suites also passed on Windows. [Coordinate provenance][b-position] · [Stacked head][b-head]
- **Upstream applicability.** Directly applicable as focused model and local lookup corrections. The model test is independently reviewable, and the AI fallback can be reviewed separately against the existing galaxy interface.

### ENX-015 — Retire background UI work when source or commander changes

- **Subsystem and classification.** Desktop worker results and UI state. Bug fix and reliability improvement.
- **Upstream and Gen2 behavior.** Pending calculations could finish after the source or commander that started them was retired. Gen2 resets source-bound navigation and Logbook state and retires obsolete results for Market, Fleet and Combat as well as stale records in Colonisation and Galaxy Map views. It checks navigation scope before applying a result and extends assistant scopes with the model's source generation. This keeps old results out of a newly attached context.
- **Code and compatibility.** `nexus-app/src/app.rs`, `assistant.rs`, `ai_tools.rs`, and `pages/navigation.rs`, `pages/logbook.rs`, `pages/combat.rs` and `pages/fleet.rs`. Existing calculations can finish after retirement but their results no longer have an active consumer. GUI verification of retirement across affected views remains open.
- **Regression evidence.** The locked Windows app suite passed 290 tests with 15 ignored. Core source-generation, assistant retirement and scoped AI fixtures exercised the underlying guards. Coverage for the individual page transitions still requires native GUI verification. [Worker retirement][b-position] · [Stacked head][b-head]
- **Upstream applicability.** Depends on ENX-004 and ENX-011 for the shared scope. The worker result and view-reset changes are reviewable together against their owning page state.

### ENX-016 — Retire transient uplink attribution across source or commander changes

- **Subsystem and classification.** Uplink event forwarding and transient service state. Privacy improvement and reliability improvement.
- **Upstream and Gen2 behavior.** Transient translator state and held events could outlive the context that produced them. Gen2 closes old forwarding attachments, resets transient state when the source or commander scope changes, ignores quiet replay for live forwarding and restricts existing Inara profile snapshot evidence to the current verified session and FID. This reduces stale attribution in the inherited upload path.
- **Code and compatibility.** `nexus-app/src/sync.rs` and `core.rs`, plus `nexus-uplink/src/service.rs` and `service_tests.rs`. Persistent upload queues and their attribution policy remain outside this phase. External service authorization, Inara consent and real upload acceptance are separate.
- **Regression evidence.** A focused synthetic recording-transport test passed, covering held Market retirement and quiet events. The standalone uplink command passed 89 tests including a doctest. [Transient retirement][b-snapshot] · [Focused ownership fixtures][b-uplink] · [Stacked head][b-head]
- **Upstream applicability.** Depends on ENX-011 for generation and FID. Review the forwarding attachment and translator reset independently of persistent queue policy.

## Phase 01C — Independent application identity and trust

Phase 01C changes are implemented in draft. The verification record below distinguishes automated results from platform runtime and distribution. See [the stacked Phase 01C draft](https://github.com/Shadowgar/EliteNexus-Gen2/pull/3) for its current status.

### ENX-017 — Exact development toolchain and retained compatibility

**Class and subsystem:** Development infrastructure improvement, compatibility improvement. The workspace now directs ordinary work to Rust 1.100.0, with exact 1.95.0 compatibility and floating stable in CI. The earlier policy tracked only minimum and stable, which left the normal compiler subject to updates. The lockfile and declared minimum are unchanged. See [development policy](../CONTRIBUTING.md) and [matrix](../.github/workflows/ci.yml). [Policy commit](https://github.com/Shadowgar/EliteNexus-Gen2/commit/067d3bd). The project-local compiler choice is fork-specific. The three-lane verification pattern adapts to upstream.

### ENX-018 — Preserve launch scope during GPU recovery

**Class and subsystem:** Reliability improvement, privacy improvement. The original restart kept the reset flag. Gen2 preserves the original options, working directory and resolved profile. Screenshot runs suppress automatic restart. Fixture runs use an explicit or empty journal source and keep speech, protected game files, MCP, updates and services disabled. The [launch fixtures](../crates/nexus-app/src/launch.rs) cover arguments with spaces, repeated recovery, profile pinning and policy flags. The original argument fixture failed and the corrected fixture passed on Windows. Related guard wiring lives in [app.rs](../crates/nexus-app/src/app.rs) and [settings UI](../crates/nexus-app/src/pages/settings.rs). [Recovery commit](https://github.com/Shadowgar/EliteNexus-Gen2/commit/03694ba). The generic restart fix is independently reviewable upstream. Gen2-specific startup and service flags require adaptation. Native device-loss recovery is outstanding.

### ENX-019 — Separate executable, profiles and local endpoints

**Class and subsystem:** Fork-specific change, architecture improvement. The original identity targeted its own executable, settings path and local endpoint. Gen2 gives those same surfaces dedicated names so both applications can coexist. The [profile resolver](../crates/nexus-app/src/settings.rs), [app identity](../crates/nexus-app/src/identity.rs), [MCP](../crates/nexus-app/src/mcp.rs) and [packaging](../tools/build-release.ps1) carry the new identity. The profile fixture preserves an upstream marker and the export fixture verifies the app header and shared format. Journal reads remain shareable. Profile migration is manual, copy-based and reversible. [Identity commit](https://github.com/Shadowgar/EliteNexus-Gen2/commit/c04dc49) ? [Export fixture](https://github.com/Shadowgar/EliteNexus-Gen2/commit/244a1f7). Names and paths are fork-specific. The profile collision checks adapt to downstream builds. Native coexistence on each OS is outstanding.

### ENX-020 — Bind updates to the Gen2 product and signing identity

**Class and subsystem:** Security improvement, compatibility improvement, fork-specific change. Upstream trusts its own signed artifacts and release workflow. Gen2 uses its own signed product and repository fields, canonical version and assets, authenticated archive size and digest, and a staged executable digest recheck. The [verification and install code](../crates/nexus-update/src) keeps the Windows rollback path and app data isolation. The [app gate](../crates/nexus-app/src/update.rs) stays closed until a production Gen2 trust root is approved. Synthetic fixtures cover upstream and valid Gen2 signatures, tampering, version eligibility, interrupted staging and rollback. [Update commit](https://github.com/Shadowgar/EliteNexus-Gen2/commit/d045b32). Signed product binding and staged validation adapt upstream. Gen2 keys and release policy are fork-specific. Signing, packaging acceptance and production distribution remain outstanding.

### ENX-021 — Authorize external operations by service and scope

**Class and subsystem:** Privacy improvement, security improvement, fork-specific change. Upstream service identities apply to the original app. Gen2 retains local Ask on loopback, opted-in public EDDN receive and documented public Spansh data reads. Gen2 keeps outbound telemetry, hosted providers and unapproved search and planner calls gated. The [assistant policy](../crates/nexus-app/src/assistant.rs), [Spansh request boundary](../crates/nexus-spansh/src/client.rs) and [outbound startup gate](../crates/nexus-app/src/sync.rs) enforce that distinction. Fixtures check client denial before transport and Local host eligibility. [Service commit](https://github.com/Shadowgar/EliteNexus-Gen2/commit/e334dda). Service policy is fork-specific. Upstream could adapt the explicit transport boundary. Live authorization and production service acceptance are outstanding.

### ENX-022 — Require separate Inara current-session snapshot consent

**Class and subsystem:** Privacy improvement, user experience improvement. The original live-sync disclosure omitted that current state can come from earlier in the game session. Gen2 describes the data, timing and withdrawal controls in [Accounts](../crates/nexus-app/src/pages/accounts.rs) and requires separate consent. The [uplink](../crates/nexus-uplink/src/service.rs) checks consent, key and pause state for snapshot admission and queueing. Recording-transport tests cover startup state, rekeying, pause, resume and revocation. [Consent commit](https://github.com/Shadowgar/EliteNexus-Gen2/commit/e334dda) and [configuration generation follow-up](https://github.com/Shadowgar/EliteNexus-Gen2/commit/a6b6d11). The disclosure and opt-in design adapt upstream. Current-session sourcing uses ENX-011 ownership. Live Inara and broader persisted queue decisions remain open.

### Phase 01C automated verification

[Windows verification head](https://github.com/Shadowgar/EliteNexus-Gen2/commit/a6b6d11fb89c37efff956711ad4d2e0f453d58e4) passed the locked application build, strict workspace Clippy and all workspace tests on exact Rust 1.100.0 and 1.95.0 MSVC. Each toolchain reported 874 passed, zero failures and 18 ignored. The Phase 01B baseline was 858 passed, zero failures and 18 ignored. Focused checks passed for launch recovery (4), Inara (10), isolated updater entry (1), Spansh access (2) and profile separation (1). The update crate passed 28 synthetic manifest, version, archive and installer tests. The Windows binary reports the Gen2 product and filename while retaining upstream copyright.

Native GUI and GPU-device-loss verification, Linux and macOS acceptance, production signing and live service operation remain outstanding. The Phase 01C draft has passed automated Windows checks. Owner acceptance, merge and release are separate statuses.

## Upstream review opportunities

The smallest standalone reviews are Windows credential protection failure handling (ENX-003), locked build and minimum-Rust CI policy (ENX-007), byte framing and numeric repair (ENX-010), and coordinate provenance (ENX-014). Credential destination handling (ENX-001 and ENX-002) and assistant and MCP retirement (ENX-004 to ENX-006) benefit from review as related groups. For journal work, the reader and watcher (ENX-008 and ENX-009) underpin source and session ownership (ENX-011), which in turn supports scoped history, qualified companions, background-result retirement and transient uplink retirement (ENX-012, ENX-013, ENX-015 and ENX-016). Each entry identifies the files and fixtures needed to review its behavior in upstream's architecture.

[a-pr]: https://github.com/Shadowgar/EliteNexus-Gen2/pull/1
[a-ask]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/0481450e5159185090591a1527a50470484661df
[a-mcp]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/2dcac87214bf3cc1cb8d94c2fe07f6f16bff5c39
[a-epoch]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/cc0738c02720b80e4a522b01d95efb9fd7c3c96d
[a-ci]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/ef73a716534e88b5016672eb87cd41c8fb651b64
[a-lint]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/a26d340b29ba531a4acdb4b7f0f1450e2ed4a139
[a-head]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/566b2cf8e02f6fdb49cee89ddf9791721225693a
[b-pr]: https://github.com/Shadowgar/EliteNexus-Gen2/pull/2
[b-replay]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/281a748bede017e2050e2471b4f81dfbda8c9724
[b-snapshot]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/f7894e3a6742cf4f9a972e8b9581e1d04293d194
[b-session]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/32652b032c1d9f7c7d16763b0fbe8cade22aa55f
[b-uplink]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/40b3ba170ad4fa6edaf07b3e61ccfb980f848db8
[b-position]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/920d775bb7742549a2dd275861ee960390acb601
[b-log]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/eb8845ccd4c6815485dc934c838970e2a7bce352
[b-head]: https://github.com/Shadowgar/EliteNexus-Gen2/commit/ffa123193f0167cc062ab2c7454898351666d563
