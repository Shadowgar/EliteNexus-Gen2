# Development resource budget

Project-local [Codex configuration](../.codex/config.toml) uses documented settings for low reasoning, 64K automatic compaction threshold, 2K stored tool-output and skills-catalog budgets, standard service tier, disabled delegation, disabled goal continuation and disabled Fast/Ultrafast controls. These are new-session defaults for trusted projects, not an enforced total token or quota ceiling. The installed client, explicit overrides, selected model and higher-priority policies can affect behavior. Do not change global account configuration or sandbox permissions.

Official configuration reference: https://developers.openai.com/codex/config-reference/

## Budget and stopping

Use a fresh task session. Start with the smallest capable available model at low effort and Standard speed; increase effort only for justified complexity. Do not resume the old foundation audit. Initial targets are 1–3 percentage points of the five-hour allowance for a minor correction, 3–5 for a small feature, 5–10 for a moderate feature and 10–15 for a complex slice. These are operator targets, not guarantees or automatic stops. Record the same account/window's starting allowance; stop at a safe checkpoint when the agreed budget is reached or exceeded. No extra prompts merely to monitor usage.

An assignment needs only: objective, relevant scope, one verification method, budget and stop condition. Return a concise handoff. Do not run additional agents or automated monitoring.

## Local operations (ordinary PowerShell)

Run from the active repository; the toolchain must already be installed. Keep the same Cargo target for incremental builds.

```powershell
.\tools\dev\Dev.ps1 Check
.\tools\dev\Dev.ps1 Check -Package nexus-update -Lint
.\tools\dev\Dev.ps1 Test -Package nexus-update -Filter notes::tests::the_real_changelog_reads
.\tools\dev\Dev.ps1 Build -Release
.\tools\dev\Dev.ps1 Run -Release -JournalDir "$env:USERPROFILE\Saved Games\Frontier Developments\Elite Dangerous"
.\tools\dev\Dev.ps1 Usage
```

Choose only the command needed. Run never builds automatically and always uses an isolated muted launch. Test requires an explicit filter or the explicit all-package option. Usage reads local JSONL counters using Python 3.10+, defaults to the latest modified session, or accepts explicit paths via `-SessionLog`. It does not export messages or infer billed quota. For comparison use the exact fresh session(s), and track quota snapshots separately when an account/window changes. Do not commit logs or private data.

## First measurement

Record actual model/effort, starting and ending allowance, elapsed duration, counter advances, total processed and cached/uncached input for DEV-01. Compare like-for-like work, not merely aggregate token counters. If the client does not apply the project settings, inspect the effective configuration without running another broad task. Preserve current data and release gates. No savings claim until measured.
