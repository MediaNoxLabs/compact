use compact_rust_choose_field_fixture::pure_circuits::{call_sum, choose, sum_with_local};
use midnight_compact_runtime::Field;

#[test]
fn generated_conditional_selects_each_field_branch() {
    let left = Field::from(7_u64);
    let right = Field::from(19_u64);
    assert_eq!(choose(true, left, right).unwrap(), left);
    assert_eq!(choose(false, left, right).unwrap(), right);
    assert_eq!(
        sum_with_local(left, right).unwrap(),
        left + right + left + right
    );
    assert_eq!(call_sum(left, right).unwrap(), left + right + left + right);
}
