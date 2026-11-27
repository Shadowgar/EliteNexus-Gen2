# Contributing to Praetorium

This page is for anyone who wants to build Praetorium from the source, report a bug well, or change the code. If you
just want to use the app, the [README](README.md) and the [user guide](docs/USER_GUIDE.md) are for you.

## Building from source

Use exact Rust **1.100.0** for Praetorium development and initial verification. Rust **1.95.0** remains the verified Windows minimum. Routine CI uses only the pinned development toolchain. Compatibility checks are available through the manual verification workflow.

Install exact Rust 1.100.0 from [rustup.rs](https://rustup.rs). Windows also uses the MSVC linker.

```sh
cargo +1.100.0 build --locked -p nexus-app
cargo +1.100.0 run --locked -p nexus-app -- --isolated
```

The executable is `Praetorium` (`Praetorium.exe` on Windows). The Windows and macOS developer packaging scripts use Praetorium names. Praetorium resolves a portable `praetorium-data` profile first, then the per-user `Praetorium` directory. macOS bundles use Application Support. `PRAETORIUM_HOME` selects a dedicated profile. Isolated runs without the override get a fresh scratch directory.

## Command-line options

| Option | What it does |
|---|---|
| `--page <name>` | Start on a sidebar page (`Dashboard`, `Exploration`, `Exobiology`, `Navigation`, `Market`, `Settings`, …). |
| `--tab <name>` | Open a tab on that page: Market sections (`abnormal`, `database`, …), the whole-galaxy map (`galaxy`), Exploration's `find`, `history` and `unsold`, Exobiology › Find planets (`untouched`, `known`), Fleet › Where to buy (`buy`), Mining › Where to mine (`where`, `where:<material>` such as `where:Monazite`, `where-route`), Navigation's planners (`carrier`, `exact`, `neutron`; `route-carrier` plots a route for your ship to your carrier on the first frame), HUD & cockpit's `glare` and `cockpit`, or a HUD colours preset to start from (`mocha`, `royalpurple`, …), with `,hue:<degrees>` setting the Colour slider on top of it (`mocha,hue:120`; 0 to 360). |
| `--journal-dir <dir>` | Read an explicit journal folder in isolated mode. |
| `--isolated` | Use a dedicated fixture profile and explicit or empty journal folder. Speech, service traffic, game discovery, updates and MCP are unavailable. |
| `--mute` | No voice. |
| `--demo` | Add a sample Ask conversation, an alert banner and sample alerts in the alert history (for screenshots); with `--tab bell` the history is open, the list under the toolbar's bell. |
| `--overlay <moment>` | The overlay with sample content: the main window shows the moment compact and expanded, see-through, as its own window (see-through off) and in VR's panel, and (except in a screenshot run) the overlay's own window turns between the two every four seconds. Moments: `jump`, `hyperspace`, `arrival`, `approach`, `sampling`, `close`, `mining`, `trade`, `docking`, `docked`, `carrier`, `two`, `alert`, `quiet`. |
| `--screenshot <png> [--shot-delay <s>] [--size <w>x<h>]` | Render the page off-screen after loading, save the PNG and exit. The window is never visible and nothing is uploaded. `--size 1000x760` checks a narrow layout (the size is in points: on a display scaled above 100 % the PNG comes out that much larger, with the same layout, and `tools\dev\shot.ps1` says so); `--shot-delay 8` or more waits for local page content. |
| `--install-desktop` / `--uninstall-desktop` | Linux only: add Praetorium and its icon to the desktop's app menu, or take it out. |
| `--updated <version>` | Given by an update's Restart now to the new copy: it waits up to 15 seconds for the old copy to let go of the instance lock, says it was updated from that version, and skips the support card. |

The images in `docs/screenshots/` are inherited captures. For a new local capture, use
`Praetorium.exe --isolated --page market --tab abnormal --screenshot target\market.png --shot-delay 8 --mute`. Check a
narrow (1000 px) and a wide window whenever a layout changes.

## Checks

Use the [local development commands and resource budget](docs/DEVELOPMENT_BUDGET.md) and the [active backlog](BACKLOG.md). Project-local Codex defaults apply only in supported trusted clients. No AI is required to run the local commands.

Use [AGENTS.md](AGENTS.md) for the risk-based verification policy. For routine work, pick the relevant test or compile check; do not run every command below.

```sh
cargo +1.100.0 test --locked -p nexus-update --lib notes::tests::the_real_changelog_reads
cargo +1.100.0 check --locked -p nexus-app
```

Use affected-crate tests or a module filter for a regression, and expand coverage for changed interfaces and safety-critical boundaries when justified. For a local playable binary, build once with the pinned toolchain and existing Cargo target; do not rebuild simply to wait for CI. Non-executable documentation needs inspection only. Embedded documentation and version changes may require a focused regression.

A ready code PR triggers a single Windows compile check. Draft PRs do not run that job, and pushes to `main` do not trigger Rust CI. Stale routine checks are cancelled. The separate **Manual verification** workflow runs one Windows workspace job by default, three pinned-toolchain platform jobs for cross-platform acceptance, or the full nine-job compatibility matrix only on explicit selection. It runs strict Clippy and workspace tests; it is not triggered by pushes or PRs. Native packaging and runtime acceptance are separate. Do not duplicate those checks locally without a technical reason.

Keep the lockfile, test source and toolchain recorded. Use synthetic inputs and a disposable profile for model, journal, provider and game-file work. Ignored live tests stay opt-in and require service authorization. Inspect the result once rather than polling with an AI agent. Record failures, cancelled checks and unverified behavior explicitly.

## Developer tools

Each crate ships examples that work on your own journals and database:

| Command | What it does |
|---|---|
| `cargo run --release -p nexus-journal --example scan_journals` | Parse statistics over your journal folder. |
| `cargo run --release -p nexus-model --example summary` | Everything the model knows about your commander. |
| `cargo run --release -p nexus-journal --example replay -- <journal> <dir> [speed]` | Replay a journal into a folder to simulate a live session. |
| `cargo run --release -p nexus-model --example predictions` | Exobiology predictions compared with what you actually sampled. |
| `cargo run --release -p nexus-uplink --example validate` | Every upload your journals would produce, checked against the EDDN schemas. Sends nothing. |
| `cargo run --release -p nexus-galaxy --example seed_journal -- <db>` | Build a galaxy database from your journals. |
| `cargo run -p nexus-eddn --example live -- <db> <seconds>` | Watch the live EDDN feed flow into a database. |
| `cargo +1.100.0 run --locked -p nexus-update --example update_sign -- manifest <dir> <tag> <date>` | Write an unsigned Praetorium-format manifest for staged test artifacts. Production signing stays disabled. |

**Testing updates.** `ELITE_NEXUS_UPDATE_URL` points a build at a folder (or an address) holding `update.json`,
`update.json.minisig`, the archives it names and optionally a `CHANGELOG.md`, instead of GitHub; the signature is still
checked against the built-in key, so sign the test manifest with `update_sign`. With the variable set even a debug build
checks by itself; it installs only from a release build outside a `target` folder (copy `Praetorium.exe` elsewhere),
and never in a test run. `--tab update` (and `update-blocked`, `update-download`, `update-ready`, `update-failed`,
`update-notes`, `updated`) shows the banner's states in a screenshot; with `--page Settings`, the About rows too.

## How it stays fast

Praetorium is pure Rust: no Python, no Node.js, no browser engine, one executable with a GPU-rendered interface
(egui/wgpu). Measured on one development PC:

| | Praetorium |
|---|---|
| Full journal history load | **~40–60 ms** for 16.5k events (parallel parse, ~260 MB/s, 600k events/s); EDDiscovery takes minutes on big histories |
| Live latency | journal + Status.json polled every **100 ms** (no missed writes, no notification lag) |
| Market data | **direct EDDN firehose** (~550 msgs/min) into a local SQLite database: prices the moment any commander docks, before websites ingest them |
| Market queries | best price within 50 ly in **~1 ms**, galaxy-wide best price 0.2 ms, trade routes in 8–22 ms on an 80k-station / 4.4M-price database |
| Startup to usable | about one second |

Keep it that way: long work (parsing, database queries, HTTP) runs off the UI thread and is polled each frame, and
the galaxy database has one writer thread with pooled readers. `docs/ARCHITECTURE.md` describes the crates and threads.

## Reporting a bug

For Praetorium, use the [Praetorium issue tracker](https://github.com/Shadowgar/Praetorium/issues) and include:

* the version (Settings › About), your OS, and how the game runs there (natively, Proton, Wine, CrossOver, Whisky);
* what you did, what you expected and what happened;
* `praetorium.log` and, if the app closed unexpectedly, `crash.log`, both from the data folder (Settings › About › Data
  folder › Open). Neither is sent anywhere by itself.

What works on Linux and what doesn't yet is collected in [discussion #17](https://github.com/uruskan/EliteNexus/discussions/17).

## Sending changes

Follow the single-branch, single-PR and resource-budget policy in [AGENTS.md](AGENTS.md). Do not open another implementation PR before the current one is resolved. Review upstream changes explicitly. See the [licensing boundary](docs/LICENSING.md), [upstream additional terms](LICENSE-ADDITIONAL-TERMS.md) and [naming policy](NAME-POLICY.md). Praetorium has an independent SemVer in the workspace manifest; synchronize the workspace lock entries and changelog for version changes, without changing third-party dependency versions.

* Read [AGENTS.md](AGENTS.md) first: the conventions (no `unwrap()` on external data, `#[serde(default)]` on external
  structs, UI text that says where data comes from and what it can't show, limits that are real, uploads that stay
  opt-in), the build and UI checks, commits and releases.
* Keep pull requests to one logical change. Commit subjects are imperative and about 70 characters, often "Area: what
  changed"; the body says what and why in plain words.
* When behaviour changes, update `docs/USER_GUIDE.md` and `CHANGELOG.md` (under "Unreleased") in the same commit. The
  user guide is compiled into the in-app assistant's knowledge base, so it has to stay true. The website in `site/`
  and its `llms.txt` describe features too.
* Build pages from `widgets.rs` and `theme.rs` (no raw egui styling, no text below 12 pt), and check the UI with a
  screenshot run at a narrow and a wide size.
* Don't run `cargo fmt` over the workspace: much of the code is hand-formatted with long lines. Match the surrounding
  style.
* Never commit `nexus-data`, databases, logs or keys (`.gitignore` covers them; don't force-add).
* Before touching an integration, read its spec in `docs/research/` (EDDN, EDSM, Inara, CAPI, Spansh, the journal,
  engineering, exploration, HUD colours and cockpit lights). The live EDDN service differs from its GitHub docs, for
  one.
* Contributions are licensed under GPL-3.0-or-later like the rest of the code. Data taken from community sources is
  credited in [NOTICE.md](NOTICE.md); add a row there when you bring in more.

## Releases and upstream review

Praetorium builds are development artifacts. Publication and automatic updates require an approved Praetorium signing identity and release process. Use signed synthetic manifests and disposable installs to verify update trust and rollback.

Every substantive change includes a cumulative [improvement-register entry](docs/GEN2_IMPROVEMENTS.md) with code and PR links, regression evidence, actual results, known limitations and upstream applicability. Update the changelog, architecture and user guide as applicable. Keep independently useful changes focused.

### Profile migration and rollback

Use the [copy-based profile migration](docs/PROFILE_MIGRATION.md) with synthetic data when testing coexistence or rollback.

## Accounts and APIs

Inara and Frontier CAPI require separate project authorization. Public EDDN receive and documented Spansh reads are available under their existing restrictions. Outbound uploads and hosted AI remain disabled. Use the synthetic uplink tests to check schemas and authorization behavior.

## Where things are

- [Architecture](docs/ARCHITECTURE.md) and [design](docs/DESIGN.md).
- [User guide](docs/USER_GUIDE.md) and [changelog](CHANGELOG.md).
- [Local website preview](tools/site/README.md).
- [Third-party notices](NOTICE.md) and [licensing](docs/LICENSING.md).
