# Changelog

All notable changes to rigcraft are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Forge scaffold for the `rust-web` profile.
- Inventory import and correction: UTF-8 CSV preview with field-coded errors and column mapping, explicit duplicate merge/replace/keep-separate decisions, immutable SQLite snapshots with per-field provenance, corrections that create new revisions, and the `/inventory` React flow.
- Evidence-backed compatibility: `/builds/check` returns compatible, incompatible or unknown per versioned rule with compared values and provenance, specific follow-ups for missing facts, conflict-over-unknown composition, revision-bound persisted reports, and a stale verdict after a part or snapshot change.
- Sample CSV download, measured token contrast check, Rust/Vitest tests, browser and Lighthouse accessibility evidence.
