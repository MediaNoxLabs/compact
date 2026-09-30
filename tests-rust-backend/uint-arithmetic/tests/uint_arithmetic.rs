use compact_rust_uint_arithmetic_fixture::pure_circuits::{multiply_uint, subtract_uint};
use midnight_compact_runtime::{BoundedUint, CompactError};

#[test]
fn generated_uint_arithmetic_checks_bounds_and_underflow() {
    let seven = BoundedUint::<255>::new(7).unwrap();
    let five = BoundedUint::<255>::new(5).unwrap();
    assert_eq!(subtract_uint(seven, five).unwrap().value(), 2);
    assert_eq!(
        subtract_uint(five, seven),
        Err(CompactError::UnsignedUnderflow)
    );
    assert_eq!(multiply_uint(seven, five).unwrap().value(), 35);
    let max = BoundedUint::<255>::new(255).unwrap();
    assert_eq!(multiply_uint(max, max).unwrap().value(), 65_025);
}
