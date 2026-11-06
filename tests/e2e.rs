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
