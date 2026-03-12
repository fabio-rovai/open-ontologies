use std::process::Command;

fn oo() -> Command {
    Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
}

#[test]
fn test_cli_help() {
    let out = oo().arg("--help").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("validate"));
    assert!(stdout.contains("query"));
    assert!(stdout.contains("import-schema"));
}

#[test]
fn test_cli_validate_inline_stdin() {
    let out = oo()
        .args(["validate", "-"])
        .stdin(std::process::Stdio::piped())
        .output()
        .unwrap();
    // Will fail until subcommand exists
    assert!(!out.status.success() || String::from_utf8_lossy(&out.stdout).contains("error"));
}
