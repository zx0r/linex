# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-11-07

### Added

- Native uptime telemetry widget (`linex uptime`) with zero-allocation human-readable formatting (`days`, `h`, `m`).
- Full porcelain v2 Git status telemetry with Catppuccin 256-color palette without color collisions.
- Symmetrical architecture for standalone widget rendering (`cpu`, `mem`, `ssd`, `uptime`, `git`).

### Changed

- Replaced circle staged indicator with distinct `✚` (`\u{271a}`) green symbol.
- Disentangled terminal colors across branch, sha, operations, and status counters.
- Strictly decoupled worktree dirty state (`*`) from index timestamp heuristics.

## [0.1.1] - 2026-11-07

### Added

- Initial release of **linex**: zero-dependency, zero-allocation telemetry engine for tmux and
  status lines.
- Hardware and VCS probes: CPU, memory, SSD, and Git porcelain v2 with SWR caching.
- Native tmux formatting dialect by default, with ANSI 256-color and plain text support.

[0.1.2]: https://github.com/zx0r/linex/releases/tag/v0.1.2
[0.1.1]: https://github.com/zx0r/linex/releases/tag/v0.1.1
