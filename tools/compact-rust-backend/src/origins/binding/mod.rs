//! Borrowed occurrence/schema binding; no semantic or diagnostic authority.
pub(crate) use crate::ir;
pub(crate) mod addresses;
mod binder;
pub(crate) mod identity;
mod schema;
mod shape;
pub(crate) use binder::{BoundOccurrences, bind};
pub(crate) use identity::{Owner, Slot};
pub(crate) const IR_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_ADDRESSES: usize = 131072;
pub(crate) const MAX_PENDING: usize = 4096;
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    IrBytes,
    Json(String),
    Addresses(&'static str),
    Shape {
        address: usize,
        reason: &'static str,
    },
    Typed {
        address: usize,
        field: &'static str,
        reason: &'static str,
    },
    Pending,
}
