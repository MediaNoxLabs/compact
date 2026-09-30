use compact_rust_uint_identity_fixture::pure_circuits::{
    add_uint, max_uint, seven, uint_identity, zero,
};
use midnight_compact_runtime::BoundedUint;

#[test]
fn generated_signature_preserves_the_compact_maximum() {
    // Compact Uint<8> is an eight-bit integer, so its inclusive maximum is 255.
    let value = BoundedUint::<255>::new(255).unwrap();
    assert_eq!(uint_identity(value).unwrap(), value);
    assert!(BoundedUint::<255>::new(256).is_err());
    assert_eq!(seven().unwrap().value(), 7);
    assert_eq!(max_uint().unwrap().value(), 255);
    assert_eq!(zero().unwrap().value(), 0);
    let sum = add_uint(
        BoundedUint::<255>::new(255).unwrap(),
        BoundedUint::<255>::new(255).unwrap(),
    )
    .unwrap();
    assert_eq!(sum.value(), 510);
}
