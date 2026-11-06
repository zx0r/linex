use std::process::Command;

#[test]
fn test_cli_all_widgets() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .arg("all")
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
    assert!(stdout.starts_with("CPU"));
}

#[test]
fn test_cli_no_color_has_no_escape_codes() {
    let output = Command::new(env!("CARGO_BIN_EXE_statty"))
        .args(["--no-color", "all"])
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
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
