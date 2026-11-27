# Praetorium changelog

Independent semantic versions. Development milestones are not public releases. The original Elite Nexus release history remains intact in [UPSTREAM-CHANGELOG.md](docs/UPSTREAM-CHANGELOG.md).

## Unreleased

- Market and station updates now track timestamps and sources separately for station details and commodity listings. Incomplete reports preserve existing listings, and older observations cannot roll back carrier location or docking access.

- Trade routes respect a known zero-credit balance. Market leaves affordability unchecked when the balance is unknown, and Ask can use an explicit hypothetical budget.

- Carrier bank transfers reconcile commander credits with the reported balance, including zero. Transfers without a reported balance continue to use the deposit or withdrawal amount.

## [0.1.0-alpha.1] - Development milestone (verification pending)

First independent Praetorium version; based on the audited Elite Nexus 0.8.2 source lineage. No public release or production update is authorized.

- **Identity:** Praetorium is the new development name. It uses original icons, an independent executable, application ID, profile and MCP transport. Settings shows the Elite Nexus attribution. The optional profile migration copies supported local data while preserving the earlier installations. Production updates and publication remain disabled.

- **Market database:** Report failed writes and incomplete import checkpoints. Compatible older databases upgrade in place. Newer or unreadable databases are preserved for recovery.
- **HUD and cockpit:** Verify the selected installation and file contents before saving or installing. Preserve inherited tone settings and unmanaged files. The controls list shows axis conflicts when filtered to conflicts.
- **Outbound sync:** Retire queued observations at consent, source and commander boundaries. Previous-run queues require fresh observations in the new session. Production uploads remain disabled.

- GPU recovery preserves original launch options and the resolved Gen2 profile. Isolated runs keep speech, network services, MCP and game-file access disabled across restart.
- Gen2 uses its own executable, data directory and MCP identity. Existing Elite Nexus profiles stay separate.
- Public EDDN receive and documented Spansh public reads are available in ordinary runs. Outbound community sync, hosted Ask providers and unapproved service operations remain gated. Inara has separate, revocable consent for current-session profile snapshots.

- **Updates:** EliteNexus-Gen2 checks, downloads and installs are disabled pending approval of Gen2 signing.
  Local Windows and macOS development packages carry the separate Gen2 identity.

- **Journals:** Keep complete records in order when new parts appear and rebuild local views when a journal is replaced.
  Logbook's **All commanders** tab shows the aggregate source. Ask and AI agents can search records attributed to
  the active verified commander. Route-writing tools remain unavailable through AI.
- **Current state:** Reject old or unqualified companion snapshots and clear coordinates after a move when the destination
  address has no position. Folder and commander changes retire pending plans and replies.

- **Ask:** Save credentials for each model service address. Earlier keys need explicit confirmation before use.
  Ask starts a fresh conversation when its model connection, key, commander or journal source changes.
  Ask and AI agents offer current-state tools while historical and route-writing tools are unavailable.
- **Credential storage:** Windows keeps the existing credential when it cannot protect a new key.
