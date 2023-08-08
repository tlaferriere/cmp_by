#[cfg(not(nightly))]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass_*.rs");
    t.compile_fail("tests/ui/fail_*.rs");
}

#[cfg(nightly)]
#[test]
fn nightly_ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/nightly_pass_*.rs");
    t.compile_fail("tests/ui/nightly_fail_*.rs");
}
