use compact_rust_bytes_identity_fixture::pure_circuits::bytes_identity;
use midnight_base_crypto::fab::{
    Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom,
};

#[test]
fn generated_bytes_use_ledger_fixed_array_alignment_and_normalization() {
    let value = [1, 2, 0, 0];
    assert_eq!(bytes_identity(value).unwrap(), value);
    assert_eq!(
        <[u8; 4]>::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 4 })
    );
    assert_eq!(
        AlignedValue::from(value).value,
        Value(vec![ValueAtom(vec![1, 2])])
    );
}
