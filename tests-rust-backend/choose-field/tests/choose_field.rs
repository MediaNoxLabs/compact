use compact_rust_choose_field_fixture::pure_circuits::choose;
use midnight_compact_runtime::Field;

#[test]
fn generated_conditional_selects_each_field_branch() {
    let left = Field::from(7_u64);
    let right = Field::from(19_u64);
    assert_eq!(choose(true, left, right).unwrap(), left);
    assert_eq!(choose(false, left, right).unwrap(), right);
}
