use std::process::Command;

#[test]
fn release_manifest_matches_package_version() {
    let expected = format!("{{\n  \".\": \"{}\"\n}}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        include_str!("../.release-please-manifest.json"),
        expected.as_str(),
        "Release Please manifest version must match the Cargo package version"
    );
}

#[test]
fn cli_reports_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_sql-tdg"))
        .arg("--version")
        .output()
        .expect("compiled sql-tdg binary should execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("version output should be UTF-8"),
        format!("sql-tdg {}\n", env!("CARGO_PKG_VERSION"))
    );
}
