# Praetorium — An Intelligent Shipboard Command Platform for Elite Dangerous

**All systems. One command.**

*Omnia sub uno imperio.*

**Independent development version: `0.1.0-alpha.1` — not a public release.** Based on the Elite Nexus 0.8.2 source lineage. The workspace manifest is the authoritative version source.

Praetorium is an independent, open-source project that aims to bring together the capabilities of the Elite Dangerous third-party ecosystem into a comprehensive, intelligent and interconnected command platform.

This application began as a modified version of Elite Nexus, created by CMDR Alka Uruskan and its contributors. Praetorium is a separate project and is not an official Elite Nexus release, successor or endorsed product.

## Our foundation

We discovered Elite Nexus while looking for a technical foundation for a broader project. Its native Rust architecture, attention to performance and breadth of existing functionality stood out. We chose to build on the work already done by CMDR Alka Uruskan and contributors. We are grateful to them and to the Elite Nexus community for the care and time they invested.

Praetorium is separate because its intended direction is different, not because we consider Elite Nexus inadequate or lacking. We do not expect its maintainer to change the original project's priorities to accommodate our vision. We want to connect capabilities from multiple community tools and services through a shared command platform. That goal may lead to substantial changes to the architecture, interfaces and workflows. Keeping the projects separate lets each develop according to its own goals. As Praetorium grows, its implementation and user experience may look quite different from the initial application. We will preserve its attribution and document improvements that may be useful upstream.

## What we aim to build

- Context-aware conversations about the current game session, with optional OpenAI and other supported model integrations.
- Planning and decision support across trading and economics, fleet carrier management and logistics, exploration and exobiology, mining, engineering, ship fitting and resource planning.
- Navigation and galaxy intelligence that can connect those workflows with current observations.
- VoiceAttack interoperability, desktop, tablet and VR command interfaces, and context-aware visual assistance.

These are development goals. Some gameplay views already exist in the inherited application. The unified workflows, OpenAI integration and additional command interfaces remain planned work.

## How we work with the community

We aim to use supported APIs, compatible open-source components, appropriately licensed algorithms and data, and interoperability with existing applications. Where direct reuse is unsuitable, we will implement the capability independently. We respect licenses, service terms, data rights, game rules and intellectual property. Integrating with another tool does not imply its developer endorses Praetorium. We will make generally applicable changes available for upstream consideration.

## Development status

The inherited application provides journal monitoring, commander and ship views, navigation, market data, fleet and carrier views, exploration, exobiology, mining, engineering and a native desktop interface. Subsequent project work covers credential handling, source and commander separation, application identity, durable database writes, protected file handling and authorization of queued observations. See the [improvement register](docs/GEN2_IMPROVEMENTS.md) for evidence and upstream applicability.

Phase 01E passed the locked Windows application build, strict workspace Clippy and 892 tests on both exact Rust 1.100.0 and 1.95.0 MSVC, with zero failures and 18 ignored. Isolated Windows checks covered branding, journal monitoring, profile migration and rollback. The owner previously passed 21 of 21 Windows gameplay checks on the build then named EliteNexus-Gen2. The hosted matrix covers Windows, Linux and macOS using both pinned toolchains and floating stable. See the [verification history](https://github.com/Shadowgar/Praetorium/pull/5) for verification and the [contributor instructions](CONTRIBUTING.md) for locked builds and tests.

The independent version and changelog correction were integrated at the owner's direction after the final hosted application run was cancelled. Earlier verification applies to the preceding source, not the entire consolidated revision. Full acceptance of the consolidated version and native packaging remains open.

Production uploads and updates, hosted AI and other unapproved service operations are disabled. Isolated runs also block protected game-file operations. Native device-loss recovery, independent review, cross-platform runtime, live-service acceptance, signing, packaging and distribution remain open. There is no public production release.

## Acknowledgments and licensing

Thank you to **CMDR Alka Uruskan** and the **Elite Nexus contributors**. Visit the [original project](https://github.com/uruskan/EliteNexus), read the [Elite Nexus website](https://uruskan.github.io/EliteNexus/) and consider [supporting the original developer](https://github.com/sponsors/uruskan).

Praetorium is licensed under [GPL-3.0-or-later](LICENSE). See [licensing and origin](docs/LICENSING.md), [third-party notices](NOTICE.md) and [historical contributor acknowledgments](docs/UPSTREAM-ACKNOWLEDGMENTS.md). The [profile migration guide](docs/PROFILE_MIGRATION.md) covers upgrading from a development profile. Elite Dangerous is a trademark of Frontier Developments plc. Praetorium is an unofficial community project and has no affiliation with Frontier Developments.
