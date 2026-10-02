#[test]
fn rust_crate_bootstraps() {
    assert_eq!(env!("CARGO_PKG_NAME"), "sql-tdg");
}
