# Working on Praetorium

Praetorium is a native Rust (egui) development project for Windows, Linux and macOS, based on Elite Nexus by CMDR Alka Uruskan and contributors. The following rules protect players' games and data.

## Development scope and resource budget

* One active implementation objective, one feature branch from verified `main`, and one open implementation PR at a time. Do not stack milestones. Preserve commit history; merge only after owner review and required checks, then retire the merged branch when safe. Existing historical drafts may be consolidated as a one-time transition.
* Start independent assignments in fresh sessions with a concise handoff. Read only the files and documentation relevant to the authorized task; do not repeat the foundation audit. Keep tool output and diffs bounded to relevant evidence.
* No subagents, parallel AI reviewers or model/reasoning escalation without explicit owner approval. No repeated CI, build, or test polling; start required checks once, record the command or CI link and stop at the checkpoint if they are pending. Do not use model turns merely to wait. Inspect finished logs once when asked.
* Run focused checks during implementation. Use existing results for unchanged source; do not repeat comprehensive suites without a technical reason. Hosted CI supplies the cross-platform matrix. Keep safety and acceptance gates intact. If blocked, preserve work and report the blocker instead of expanding scope.
* Stop after the authorized deliverable or a safe checkpoint. Report the branch, commit, changed paths, checks and remaining work concisely. Do not start a new assignment, publish a release or rewrite history autonomously.
* Track upstream separately; reconcile relevant changes explicitly and preserve license and attribution obligations. Do not blindly sync or import upstream release credentials or branding.
* `Cargo.toml` workspace version is the authoritative independent Praetorium SemVer. Coordinate version changes with the workspace entries in `Cargo.lock`, changelog and platform metadata; do not update third-party dependencies merely to bump the product version. Preserve the historical upstream version as provenance only. A version or tag is not authorization to publish. Before claiming a milestone verified, record the exact tested revision and distinguish pending from passed checks.

## Consult as relevant

* `docs/ARCHITECTURE.md`: crates, threads, the design system, code conventions.
* `docs/DESIGN.md`: how the app looks and why: principles, colours, type, components, charts and a checklist for new
  pages. Every UI change follows it; its last section says how far the app has moved to it.
* `docs/USER_GUIDE.md`: what every page does. It's compiled into the in-app assistant's knowledge base, together
  with `crates/nexus-ai/knowledge/*.md`, so the assistant tells users whatever it says. When behaviour changes,
  update it in the same commit.
* `site/`: the local website preview validated by `.github/workflows/pages.yml` and its `llms.txt`, which AI assistants read.
  They describe features too: keep them true when behaviour changes.
* `CHANGELOG.md`: what changed in each release. Add your user-visible changes under "Unreleased" as you go; historical release notes preserve their original attribution.
* `docs/research/`: verified specs (EDDN, EDSM, Inara, CAPI, Spansh, the journal, engineering, exploration, HUD
  colours and cockpit lights). Check them before touching an integration; the live EDDN service differs from its
  GitHub docs, for one.
* `CONTRIBUTING.md`: building, command-line options, tests, developer tools, how to report a bug and send a change.

## Hard rules

* **External services require Praetorium authorization and user opt-in.** Production uploads and updates are disabled in development drafts. Inara current-session profile state requires separate consent after service approval.
* **Protect game files.** Ordinary cockpit and HUD operations keep their explicit controls and surgical writes. Isolated runs suppress discovery and protected operations. Use synthetic journals and isolated profiles for verification. Keep Frontier shader exports outside Git.
* **Preserve app boundaries.** Use Praetorium file and platform identities. Do not copy a user's upstream credentials or app profile. Never commit application data directories, databases, logs or keys.

* Never panic on external data: no `unwrap()` on I/O or network data, `#[serde(default)]` on external structs.
* **Be honest in the UI.** Say where data comes from and what it can't show (the Exobiology bonus hunt explains why
  "no record" isn't proof; Find valuable bodies says Spansh skips populated systems). Explain, don't just show
  numbers.
* **Limits must be real.** Spansh planners use `tours::limits` (Spansh's own form bounds: jump range 10-100 ly,
  radius up to 1,000 ly, up to 2,000 stops); local database searches can reach the whole galaxy. Don't invent caps;
  where a cap protects performance, say so in a tooltip.
* Pushes, tags and releases are the maintainer's (see Releases); changes arrive as pull requests.

## Build, test, check

Develop with exact Rust **1.100.0**. The verified Windows minimum remains **1.95.0**. CI checks the pinned development toolchain, the minimum, and floating stable. Record the actual toolchain for each result.

```powershell
cargo +1.100.0 build --locked -p nexus-app
cargo +1.100.0 clippy --locked --workspace --all-targets -- -D warnings
cargo +1.100.0 test --locked --workspace
```

CI (`.github/workflows/ci.yml`) runs the same clippy and tests on `windows-latest`, `ubuntu-latest` and
`macos-latest` for every push to `main` and every pull request; keep them green. A push that changes only the website
or documents the app doesn't compile (`site/`, `tools/site/`, `AGENTS.md`, `CONTRIBUTING.md`,
`docs/ARCHITECTURE.md`, `docs/DESIGN.md`, `docs/research/`, `docs/screenshots/`) skips it; the README,
`CHANGELOG.md` (What's new after an update), `docs/USER_GUIDE.md` and `data/` are compiled in, so changes to them
still run it.

* On Windows, `tools\dev\cargo.ps1 -Log build -CargoArgs "build -p nexus-app -j 4"` runs cargo at BelowNormal
  priority and writes the output to `target\cc\<Log>.log`, so a game running on the same PC keeps its frame rate.
  It dot-sources `tools\env.ps1`, which puts a toolchain that isn't on PATH on it through a git-ignored
  `tools\env.local.ps1` (see the comment in `env.ps1`).
* Tests that need the internet are `#[ignore]`d, e.g. `cargo test -p nexus-app untouched -- --ignored --nocapture`.
* Spansh planners, live: `cargo run -p nexus-spansh --example smoke -- --only riches,exobio` (all steps: no `--only`).
* More dev tools (journal stats, model summary, replay, EDDN upload validation) are listed in `CONTRIBUTING.md`.

Use focused checks on the development toolchain during iteration. At milestone completion, repeat the locked build, strict Clippy and workspace tests with `+1.95.0`. Review compiler-floor changes explicitly and keep the dependency graph locked.

For model or settings changes, use a separate checkout and target directory. Keep the reviewed lockfile and
record the exact Rust toolchain, locked build, Clippy and test results. Use synthetic journals, isolated settings
and loopback model fixtures. Keep service uploads and update operations disabled. GUI checks need a disposable
profile or VM if automatic recovery can reopen ordinary discovery paths. Never use real keys or private data
in fixtures, logs, PRs or artifacts. Keep credential destinations and observations scoped across async work.

## Linux

The game runs under Proton or Wine there, so its journals and Options sit in a Wine prefix
(`nexus_journal::paths`: `default_journal_dir`, `options_dir`, `steam_libraries`). Keep Windows-only code behind
`#[cfg(windows)]` with a Linux counterpart (`secrets.rs`, `tts.rs`, `main.rs`), and never link a system library
the Linux binary might not find at start (that's why voice runs `spd-say` instead of the `tts` crate there).

From a Windows PC, `tools/dev/linux.sh` runs the Linux checks inside a WSL distro that has Rust, Xvfb and Mesa's
software Vulkan (an Ubuntu 24.04 WSL1 distro named `EliteNexusLinux` works; pass yours to `wsl -d`). From the
repository root:

```powershell
wsl -d EliteNexusLinux -u root -- bash tools/dev/linux.sh cargo clippy --workspace --all-targets -j 4 -- -D warnings
wsl -d EliteNexusLinux -u root -- bash tools/dev/linux.sh cargo test --workspace -j 4
wsl -d EliteNexusLinux -u root -- bash tools/dev/linux.sh shot --page Settings --out linux-settings --size 1000x760
```

Builds go to `/root/target` inside the distro. CI runs the same clippy and tests on `ubuntu-latest`.

## macOS

The game runs under NotProton (Steam Play for the macOS Steam client, built on CrossOver), CrossOver or Whisky, so
macOS shares the Linux code paths (`cfg(not(windows))`); the differences are behind `cfg!(target_os = "macos")`:
the Steam folder and the bottles in `nexus_journal::paths`, the data folder (`settings::data_dir`: never inside an
app bundle), voice through `/usr/bin/say` (`tts.rs`) and the bundle's own Dock icon (`main.rs`). Prefer `cfg!` over
`#[cfg]` there, so the Linux build type-checks the macOS branches. `tools/macos/build-app.sh` builds, packs and
ad-hoc signs `Elite Nexus.app` (`--install` copies it to Applications); its icon comes from
`crates/nexus-app/examples/macos_iconset.rs`. CI runs clippy and tests on `macos-latest`.

## Checking the UI

```powershell
powershell -File tools\dev\shot.ps1 -Page Exploration -Tab history -Out history -Size 1000x760
```

The window renders at -20000,-20000 (never visible, no taskbar button, so it never pops up over a running game),
saves `target\shots\<Out>.png` and exits; open the PNG to look at it. Check a narrow (1000 px) and a wide size when
a layout changes.

* `-Page`: a sidebar page (`Dashboard`, `Exploration`, `Exobiology`, `Navigation`, `Market`, `Settings`, ...).
* `-Tab`: `abnormal`, `database`, ... (Market), `galaxy` (whole-galaxy map), `find`, `history`, `unsold`
  (Exploration), `carrier`, `exact`, `neutron`, `route-carrier` (Navigation: that planner; `route-carrier` plots a
  route for your ship to your carrier on the first frame), `untouched`, `known` (Exobiology > Find planets), `clear-unsold` (Exobiology > This system, with the unsold list's Clear list asking to confirm), `buy` (Fleet > Where to buy, from the current
  ship's build with every row ticked, as for a second ship like it; `buy-build`: nothing loaded), `buy-one` (Fleet > Where to buy, one module: a 6A frame shift drive), `plan`, `plan-fsd`, `plan-new`, `plan-hull` (Fleet > Plan a loadout with the current ship; `plan-fsd` opens its frame shift drive's list, with engineering planned for it; `plan-new`: its start, with saved builds and every hull; `plan-hull`: a new Python; `plan-defence`, `plan-weapons`, `plan-power`, `plan-travel`, `plan-compare`: the current ship on that panel, Compare beside the hull as a shipyard sells it, Weapons against an Anaconda as sold), `materials`, `engineers`
  (`engineers:selene`: with that engineer's whole way open, any part of a name without spaces), `suits`, `synthesis`,
  `merc`, `rolls` (Engineering; `rolls`: four hull reinforcement packages with the rolls at each grade open), `where`, `where:Monazite`, `where-route` (Mining > Where to mine: near
  you, near you for that material, or along a route towards Colonia), `career`, `activity`, `ships`, `all` (Statistics), `support`, `support-close` (any page: the support card shown at
  each start, which screenshot runs otherwise never show; its countdown running, or over with Close showing), `update`,
  `update-blocked`, `update-download`, `update-ready`, `update-failed`, `update-notes`, `updated` (any page: the update
  banner's states, a next version made up, the notes from CHANGELOG.md; with `-Page Settings`, About's Updates row too), `glare`, `cockpit` or a preset name such as `mocha` (Hud: that tab, or HUD colours starting from that
  theme; add `,options` to open the protection's options, or `,hue:120` to set the Colour slider to 120° on top of
  the theme; nothing is written). `find`, `untouched`, `known`, `buy`,
  `buy-one`, the `where` tabs and `route-carrier` start their search on the first frame: add
  `-Delay 15` to `-Delay 25` for Spansh to answer. `routes` (Market › Trade routes) searches the local database on the
  first frame too, `station` (Market › Station) opens four markets side by side: your system's, then the nearest,
  `browse` opens Market › Find commodity with its search field's whole list open, and `trip` opens Market › Shopping
  trip, planned for the goods the settings hold (`market.trip`).
* HUD & cockpit with a game folder of your own making: set `ELITE_NEXUS_GEN2_GAME_DIR` to a folder holding a dummy
  `EliteDangerous64.exe`; add a `d3dx.ini` starting `; Elite Nexus` with a `[Constants]` section (`x1 = 1` switches
  the HUD colour protection on) and an `EliteNexus-3Dmigoto.json` of `{"migoto_version":"1.4.9","files":[]}` for
  the installed state. For README shots, also point `LOCALAPPDATA` at a demo folder holding
  `Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml` (a theme, glare values):
  the tabs then show an applied state, and no real paths or capture leftovers. Delete the demo folders afterwards.
* `-Demo` adds a sample Ask conversation, an alert banner and sample alerts in the alert history; `-Tab bell` with it
  opens the history (the list under the toolbar's bell).
* `-Overlay <moment>` shows an overlay moment compact and expanded, see-through, as its own window (see-through off) and in VR's panel (`jump`, `hyperspace`, `arrival`, `approach`, `sampling`, `close`, `mining`, `trade`, `docking`, `docked`, `carrier`, `two`, `alert`, `quiet`).
  The overlay's own window never opens in a screenshot run.

egui 0.36 layout lessons learned here:

* `Grid`: set `.max_col_width(ui.available_width())`, or wrapped text keeps a column at its first-frame width. Put
  the column that wraps last; the last column gets the width that's left.
* Settings-style rows (`widgets::Group::row`) reserve the title's width, so long values truncate.
* Don't change the zoom while a slider is dragged: the slider rescales under the pointer and runs away. That's why
  "Text and controls size" uses fixed steps.
* Pages that `app.rs` doesn't wrap in a `ScrollArea` (Exploration, Navigation, Market, Logbook, ...) scroll
  themselves: a `TableBuilder` scrolls on its own, anything else needs an explicit `ScrollArea`.
* Build pages from `widgets.rs` (`cards`, `group`, `tabs`/`segmented`, `toggle`, `primary_button`, `badge`,
  `footnote`, `empty`) and `theme.rs` (`pal`, `size`): no raw egui styling, no text below 12 pt. `docs/DESIGN.md`
  says which component and which colour to use where.

## Worktree isolation

* Use a separate worktree if needed, not another active development branch, and your own `CARGO_TARGET_DIR` (`target-agents\<name>` is
  git-ignored): a shared `target\` means lock waits and half-built binaries. `tools\dev\cargo.ps1 -TargetDir ...`
  and `tools\dev\shot.ps1 -Exe ...` take it.
* Files most features touch: `crates/nexus-app/src/app.rs` (page wiring), `widgets.rs`, `theme.rs`, `ai_tools.rs`
  (the assistant's tools), `pages/navigation.rs` (`PlannedRoute`), `docs/USER_GUIDE.md`. Keep edits there small,
  commit them early, and rebase often.
* Keep shared crates compiling between edits: someone may be building against them.
* Don't run `cargo fmt` over the workspace. Much of the code is hand-formatted with long lines; a mass reformat
  creates unnecessary conflicts. Match the surrounding style.

## PowerShell 5.1 traps

* `Get-Content`/`Set-Content` use the ANSI code page by default: round-tripping a UTF-8 source through them corrupts
  `·`, `›`, `≈` and friends and adds a BOM. Use an editor tool, or `[IO.File]::ReadAllText`/`WriteAllText` with
  `New-Object System.Text.UTF8Encoding($false)`. Keep `.ps1` files ASCII.
* `git commit -m "..."` breaks on embedded double quotes: write the message to a file and `git commit -F` it.
* With `2>&1`, a native command's stderr shows up as errors even on success: check `$LASTEXITCODE`.

## Commits

* One logical change per commit. Subject in the imperative, about 70 characters, often "Area: what changed"
  (`Settings: size in fixed steps; keep row titles readable`). The body says what and why, in plain words.
* A change made with an AI tool ends with a `Co-Authored-By:` trailer naming the model.
* Update `docs/USER_GUIDE.md` and `CHANGELOG.md` in the same commit as the behaviour they describe.

## Releases

Publication, signing and production updates require separate owner approval and a Praetorium trust root. The release workflow is an informational placeholder. Local packaging uses Praetorium-named artifacts. Keep original Elite Nexus executables, profiles and signed manifests separate.

## Improvement documentation

For every substantive change, update the cumulative [improvement register](docs/GEN2_IMPROVEMENTS.md). Record affected paths, commits and PRs, regression tests and observed results, compatibility effects, upstream applicability and remaining limits. Update the user guide, changelog and architecture where their contracts change. Keep older entries and link later refinements. Group routine mechanical edits into related entries.

## Open decisions

* `GalaxyOptions::all_eddn_bodies` (keep every landable EDDN body, not just biology and valuable worlds) is off.

## Ideas queue

* A panel showing exactly what the Ask assistant can see and do, built from its tool list.
* Retake the README screenshots of Exobiology and Navigation: they predate 0.2.2's tabs. They were made from a
  replay of a real exobiology landing (`cargo run --release -p nexus-journal --example replay`). The replay writes
  only the journal: the sampling radar takes each sample's position from `Status.json` (latitude and longitude at the
  `ScanOrganic`), and the route comes from `NavRoute.json`, so a retake has to write those next to the journal too.
* A copy button next to every system name (a Reddit user's request, 0.4.0 launch thread; Inara has one). Click-to-copy
  already exists on Market, Exploration, Exobiology, Engineering, Colonisation, Galaxy map and Navigation, but it
  is invisible (a plain label with a tooltip) and each page calls `ctx.copy_text` by hand. Missing entirely: the
  Dashboard Activity feed, because `FeedItem` (`nexus-model/src/state.rs`) keeps the system name only inside `text`;
  give it a `system: Option<String>`. Plan: one shared `widgets::system_name` (name plus a copy icon, "Copied"
  flash), used on every page that shows a system; then say so in `docs/USER_GUIDE.md` and `CHANGELOG.md`. Check
  Stats, Missions, BGS, Carrier and Mining too.
* Surface compass, on-foot farming guide, installer.
