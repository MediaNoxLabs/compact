use compact_rust_one_tuple_fixture::pure_circuits::one_tuple;
use midnight_compact_runtime::Field;

#[test]
fn one_element_compact_tuple_is_a_rust_tuple() {
    let value = Field::from(7_u64);
    assert_eq!(one_tuple(value).unwrap(), (value,));
}
