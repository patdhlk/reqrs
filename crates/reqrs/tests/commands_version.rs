use reqrs::commands::version::version;

#[test]
fn version_returns_semver_like_string() {
    let v = version();
    assert!(v.chars().next().unwrap().is_ascii_digit(), "version: {v:?}");
    assert!(v.contains('.'), "expected dotted version, got {v:?}");
}
