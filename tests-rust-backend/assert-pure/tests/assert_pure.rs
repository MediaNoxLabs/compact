use compact_rust_assert_pure_fixture::pure_circuits::checked_value;
use midnight_compact_runtime::Field;

#[test]
fn pure_assertions_match_typescript_success_and_ordered_failures() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/assert-pure.json"
    ))
    .unwrap();
    assert_eq!(
        checked_value(true, true, Field::from(42_u64)).unwrap(),
        Field::from(
            oracle["pass"]["ok"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        )
    );
    for (name, first, second) in [("firstFails", false, false), ("secondFails", true, false)] {
        let error = checked_value(first, second, Field::from(42_u64)).unwrap_err();
        assert_eq!(error.to_string(), oracle[name]["error"], "{name}");
    }
}
