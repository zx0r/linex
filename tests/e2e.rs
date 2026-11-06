use std::process::Command;

#[test]
fn test_cli_all_widgets_default_icons() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('\u{f035b}'));
    assert!(stdout.contains('\u{efc5}'));
    assert!(stdout.contains('\u{f02ca}'));
}

#[test]
fn test_cli_all_widgets_no_icons() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("--no-icons")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CPU"));
    assert!(stdout.contains("MEM"));
    assert!(stdout.contains("SSD"));
}

#[test]
fn test_cli_individual_widgets() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("cpu")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('\u{f035b}'));
}

#[test]
fn test_cli_git_widget() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("git")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('\u{f418}'));
}

#[test]
fn test_cli_git_status_detailed() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["git", "-G"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('\u{f418}'));
}

#[test]
fn test_cli_no_color_has_no_escape_codes() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("--no-color")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CPU"));
    assert!(stdout.contains("MEM"));
    assert!(stdout.contains("SSD"));
}

#[test]
fn test_cli_custom_separator() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["--separator", " :: ", "cpu", "mem"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(" :: "));
}

#[test]
fn test_cli_version_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("-V")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("statty "));
}

#[test]
fn test_cli_help_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("-h")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("WIDGETS:"));
    assert!(stdout.contains("OPTIONS:"));
}

#[test]
fn test_cli_tmux_style() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["-s", "tmux", "cpu"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("#[fg=colour"));
}

#[test]
fn test_cli_ansi_style() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["-s", "ansi", "cpu"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(output.stdout.contains(&0x1b));
}

#[test]
fn test_cli_plain_style() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["-s", "plain", "cpu"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
    assert!(!output.stdout.windows(5).any(|w| w == b"#[fg="));
}

#[test]
fn test_cli_bytes_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["--bytes", "mem"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('/') && (stdout.contains('G') || stdout.contains('M')));
}

#[test]
fn test_cli_invalid_arg() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("--unknown-flag")
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
}

#[test]
fn test_lib_render_in_process() {
    let mut out = statty::fmt::Out::new();
    let temp = std::env::temp_dir();
    let cfg = statty::Config {
        widgets: statty::Widgets::ALL,
        dialect: statty::fmt::Dialect::Plain,
        icons: false,
        bytes: false,
        git_detail: false,
        sep: b" | ",
        state_dir: Some(temp.as_os_str()),
    };
    let ok = statty::render(&cfg, &mut out);
    assert!(ok);
    assert!(!out.as_bytes().is_empty());
}

#[test]
fn test_out_buffer_overflow() {
    let mut out = statty::fmt::Out::new();
    let huge = [b'a'; statty::fmt::OUT_CAP + 10];
    out.push(&huge);
    assert!(out.full());
    let mark = out.mark();
    out.rollback(mark);
    assert!(!out.full());
}
