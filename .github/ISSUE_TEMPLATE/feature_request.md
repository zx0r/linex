---
name: Feature Request
about: Propose a functional enhancement or architectural feature for statty.
title: "[FEATURE] "
labels: ["enhancement"]
assignees: ''
---

## Problem Statement / Motivation
<!-- Is your feature request related to a specific workflow problem? Describe clearly. -->

## Proposed Solution & CLI UX
<!-- Describe how the CLI syntax, widget, or output behavior should look. -->
```console
$ statty ...
```

## Architectural Considerations
- [ ] Preserves sub-millisecond execution latency.
- [ ] Strictly zero runtime dependencies (POSIX / Darwin C ABI).
- [ ] Zero heap allocation on telemetry hot path.
- [ ] Codebase remains strictly $\le 1000$ lines.

## Alternatives Considered
<!-- Any alternative implementations or workarounds you've evaluated. -->
