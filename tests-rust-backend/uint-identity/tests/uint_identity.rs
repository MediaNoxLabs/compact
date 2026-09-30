use compact_rust_uint_identity_fixture::pure_circuits::uint_identity;
use midnight_compact_runtime::BoundedUint;

#[test]
fn generated_signature_preserves_the_compact_maximum() {
    // Compact Uint<8> is an eight-bit integer, so its inclusive maximum is 255.
    let value = BoundedUint::<255>::new(255).unwrap();
    assert_eq!(uint_identity(value).unwrap(), value);
    assert!(BoundedUint::<255>::new(256).is_err());
}
