# Working on Praetorium

Praetorium is an independent Rust/egui command platform based on Elite Nexus by CMDR Alka Uruskan and contributors. These are the authoritative development rules. Consult [development reference](docs/DEVELOPMENT_REFERENCE.md) only for details relevant to the current task.

## Work in small, bounded assignments

- One objective per fresh AI session; one active implementation branch from the integrated `main` and one PR. No stacked milestones. Keep unfinished work in a draft; mark ready once the focused work is ready for review. Owner approval controls integration, branch retirement and releases. Do not rewrite history.
- Read the relevant files, not the whole repository. No repeat foundation audits, broad context dumps, or rereading every design document. Use concise diffs, targeted searches and short handoffs. Do not recursively load the improvement register or historical research.
- No subagents, parallel AI reviews, reasoning/model escalation, unrelated refactoring or new assignments without approval. Keep the task to its authorized scope. If a task grows, stop at a safe checkpoint with the remaining issue identified.
- Do not spend AI turns polling builds, tests, CI, sleeps or background commands. Use normal shell commands or GitHub Actions; record the command, log or run URL and stop if pending. Read completed results once on request. No repeated runs on unchanged code. For a failed check, diagnose the specific failure and rerun only that check after a relevant fix; if still blocked, report it instead of looping.
- Reuse a persistent local Cargo target directory for the active worktree. Do not clean caches or create a new target per milestone. Keep separate targets only for simultaneous builds, incompatible environments or deliberate isolation. Preserve the known-good executable/profile for rollback.
- At the end, report commit, changed behavior, exact checks actually run, any failure or skipped acceptance and the next action. Do not call a pending or cancelled check passed. Do not wait for a manual full matrix merely to finish a development task.

- Use the project-local `.codex/config.toml` in new trusted sessions. The [resource budget](docs/DEVELOPMENT_BUDGET.md) defines initial quota targets and the concise assignment format. Stop at the budget checkpoint; no automated goal continuation. Use [BACKLOG.md](BACKLOG.md) for the next authorized item and [ROADMAP.md](ROADMAP.md) for direction. Do not load historical research by default.

## Testing and build policy

Development toolchain: exact Rust **1.100.0**. Retained compatibility target: **1.95.0**. Record the actual toolchain and revision; use the checked-in lockfile.

| Change | Default verification |
| --- | --- |
| Non-executable documentation or metadata | Inspect the diff; no Cargo invocation. |
| Small bug fix | One relevant regression test or module filter in the affected crate. |
| Feature or interface change | Affected tests; use `cargo check --locked -p nexus-app` if application integration needs checking. |
| Security, data integrity, migration, protected writes or cross-crate contract | Relevant boundary and failure-path regressions; expand scope when justified by risk. |
| Local gameplay acceptance | Build once and run the affected workflow in an isolated profile. |
| Broader milestone / release candidate | Owner-authorized manual verification workflow at the appropriate scope. |

Use `tools/dev/Dev.ps1` for ordinary local Check, Test, Build, Run or Usage operations; see the resource budget for examples. Run only the relevant command. Direct Cargo examples (select one):

```powershell
cargo +1.100.0 test --locked -p nexus-update --lib notes::tests::the_real_changelog_reads
cargo +1.100.0 test --locked -p nexus-model --lib <filter>
cargo +1.100.0 test --locked -p nexus-app --bin Praetorium <filter>
cargo +1.100.0 check --locked -p nexus-app
cargo +1.100.0 build --locked -p nexus-app
```

Do not add tests to hit a quota or remove existing coverage to lower a count. Add meaningful regressions for changed behavior; retain safety tests. Do not automatically run workspace-wide tests, Clippy, a release build, or multiple toolchains after every change. Use focused linting when relevant. Do not treat a compile check as runtime acceptance.

Hosted routine CI is one Windows compile-only job on a ready code PR, with superseded runs cancelled. Draft PRs do not start the build job; pushes to `main` do not trigger Rust CI. Website checks are separate. Use the manual verification workflow only with owner authorization: one Windows workspace job, three-platform pinned-toolchain verification, or the full nine-job compatibility matrix. No automatic live-service tests, publishing or signing.

## Non-negotiable safety and quality

- External services require project authorization and user opt-in; production uploads, updates and hosted AI remain gated. Inara profile snapshots require separate current-session consent.
- Protect game files, credentials, profiles and player data. Use synthetic fixtures, explicit sources and disposable profiles for tests. Isolated mode blocks discovery and protected writes. Do not commit logs, databases, keys, application profiles or Frontier shader exports. Never use real credentials in test fixtures.
- No panics on external input; use fallible I/O and appropriate serde defaults. Preserve source/commander/session ownership across async work, and truthful UI states. Explain data provenance and real limits.
- Respect the GPL, third-party terms and original attribution. Review applicable integration specs before changing services; do not blindly sync upstream or copy release keys or branding. Keep original installations and trust roots separate.
- Preserve the native app's performance: no blocking work on the UI thread. Follow existing formatting, widgets and theme conventions; no workspace-wide reformat.
- `Cargo.toml` defines independent SemVer; synchronize internal lock entries and relevant version metadata, preserve historical upstream release notes separately, and do not churn dependencies for a version bump.
- Keep [user guide](docs/USER_GUIDE.md), [changelog](CHANGELOG.md) and [improvement register](docs/GEN2_IMPROVEMENTS.md) accurate for changed behavior. Record actual verification, risks and upstream applicability. Group related mechanical changes; do not produce redundant reports. Documentation alone is not acceptance.
- No release, tag, production signing, production update or external service activation without separate owner approval.

## Read only what the task needs

- [Development reference](docs/DEVELOPMENT_REFERENCE.md): platform specifics, UI checks, local tools and technical conventions.
- [Architecture](docs/ARCHITECTURE.md) / [design](docs/DESIGN.md): affected subsystem or UI only.
- [Contributor guide](CONTRIBUTING.md): local workflow and commands.
- `docs/research/`: relevant service spec only.
