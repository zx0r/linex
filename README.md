<p align="center">
  <a href="https://github.com/zx0r/linex/releases">
    <img src="https://img.shields.io/github/v/release/zx0r/linex?color=4be1ff&label=RELEASE&style=flat-square&logo=github&labelColor=1a1a1a" alt="Release" />
  </a>
  <a href="LICENSE-MIT">
    <img src="https://img.shields.io/badge/LICENSE-MIT%2FApache--2.0-9fef00?style=flat-square&logo=opensourceinitiative&logoColor=white&labelColor=1a1a1a" alt="License" />
  </a>
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/CORE-RUST_2024-orange?style=flat-square&logo=rust&logoColor=white&labelColor=1a1a1a" alt="Rust 2024" />
  </a>
</p>

<p align="center">
  <a href="https://github.com/user-attachments/assets/808dbcbd-fad3-43f3-a6c9-788406c55331">
    <img src="assets/gif/xstory-preview.gif" alt="xstory demo — sub-millisecond shell history fuzzy search and preview" style="border-radius: 8px; max-width: 100%;" />
  </a>
</p>

---

### Installation

```bash
# Cargo (crates.io)
cargo install linex

# Homebrew (macOS / Linux)
brew install zx0r/tap/linex

# Build from source (LTO fat, stripped)
gh repo clone zx0r/linex
cd linex && just release
```

---

### Integration

```bash

# Tmux
set -g status-left '#(linex git)'
set -g status-right '#(linex cpu mem ssd)'
```

---

### [ l i n e x ]

<p align="center">
  <img src="./assets/gif/xstory-crab.gif" width="360" height="" alt="xstory" />
</p>

```shell
linex [WIDGET] [OPTIONS]

[✓] zero-allocation stack formatting
[✓] sub-millisecond telemetry probes
[✓] native tmux and ANSI dialects
[✓] zero runtime dependencies

fast telemetry → rendered instantly for tmux and status lines
```

---

### License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
