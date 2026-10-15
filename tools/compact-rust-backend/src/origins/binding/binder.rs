use crate::origins::binding::{
    Error, IR_INPUT_BYTES, MAX_PENDING, Owner, Slot,
    addresses::{AddressAccess, Edge, Graph},
    ir::Contract,
    schema::{self, Member, Scalar, Slice, Value as TypedValue},
    shape,
};
use serde_json::Value;
use std::collections::HashMap;

/// All keys borrow the immutable input Contract. No owned/cloned node or pointer
/// survives its source lifetime; queries do not use recursive semantic equality.
pub struct BoundOccurrences<'ir> {
    entries: HashMap<(Owner<'ir>, Slot), usize>,
    transport_only: usize,
}
impl<'ir> BoundOccurrences<'ir> {
    pub fn address(&self, owner: Owner<'ir>, slot: Slot) -> Option<usize> {
        self.entries.get(&(owner, slot)).copied()
    }
    /// Query identity during a shorter callback reborrow, without retaining it.
    pub(crate) fn expression_address(&self, source: &super::ir::Expr) -> Option<usize> {
        self.entries
            .get(&(Owner::Expr(source), Slot::Node))
            .copied()
    }
    pub(crate) fn occurrence_address(&self, owner: Owner<'_>, slot: Slot) -> Option<usize> {
        self.entries.get(&(owner, slot)).copied()
    }
    pub fn typed_slot_count(&self) -> usize {
        self.entries.len()
    }
    pub fn transport_only_count(&self) -> usize {
        self.transport_only
    }
}

enum Task<'ir, 'raw> {
    Node(Owner<'ir>, usize, &'raw Value),
    Members(
        Owner<'ir>,
        usize,
        &'raw serde_json::Map<String, Value>,
        std::vec::IntoIter<Member<'ir>>,
    ),
    Array(
        Owner<'ir>,
        &'static str,
        usize,
        Slice<'ir>,
        usize,
        &'raw [Value],
    ),
}

/// Shape binding only. The caller separately establishes byte-pair integrity,
/// frontend issuance and semantic preflight. This API mints none of those facts.
/// Current public IR ingress is exactly four MiB; default serde recursion stays
/// enabled. A raw Value is temporary and is dropped before the index is returned.
pub fn bind<'ir, A: AddressAccess>(
    contract: &'ir Contract,
    exact_ir: &[u8],
    addresses: &A,
) -> Result<BoundOccurrences<'ir>, Error> {
    if exact_ir.len() > IR_INPUT_BYTES {
        return Err(Error::IrBytes);
    }
    let raw: Value = serde_json::from_slice(exact_ir).map_err(|e| Error::Json(e.to_string()))?;
    let graph = Graph::new(addresses)?;
    shape::verify(&raw, &graph)?;
    let mut bound = BoundOccurrences {
        entries: HashMap::new(),
        transport_only: 0,
    };
    let mut bound_addresses = vec![false; addresses.len()];
    let mut pending = vec![Task::Node(Owner::Contract(contract), 0, &raw)];
    while let Some(task) = pending.pop() {
        match task {
            Task::Node(owner, id, raw) => {
                bind_slot(&mut bound, &mut bound_addresses, owner, Slot::Node, id)?;
                let map = raw.as_object().ok_or(Error::Typed {
                    address: id,
                    field: "",
                    reason: "typed owner must be raw object",
                })?;
                let descriptor = schema::describe(owner);
                if !descriptor.unit
                    && map
                        .keys()
                        .any(|key| !descriptor.members.iter().any(|member| member.name == key))
                {
                    return Err(Error::Typed {
                        address: id,
                        field: "",
                        reason: "unknown field on non-unit typed owner",
                    });
                }
                pending.push(Task::Members(
                    owner,
                    id,
                    map,
                    descriptor.members.into_iter(),
                ));
            }
            Task::Members(owner, id, map, mut members) => {
                if let Some(member) = members.next() {
                    pending.push(Task::Members(owner, id, map, members));
                    let Some(raw) = map.get(member.name) else {
                        if member.omission {
                            continue;
                        }
                        return Err(Error::Typed {
                            address: id,
                            field: member.name,
                            reason: "missing typed field",
                        });
                    };
                    let child = graph
                        .child(id, Edge::Field(member.name))
                        .ok_or(Error::Typed {
                            address: id,
                            field: member.name,
                            reason: "missing field address",
                        })?;
                    bind_slot(
                        &mut bound,
                        &mut bound_addresses,
                        owner,
                        Slot::Field(member.name),
                        child,
                    )?;
                    value(&mut pending, owner, member.name, child, member.value, raw)?;
                }
            }
            Task::Array(owner, field, id, values, index, raw) => {
                if index < values.len() {
                    let child = graph.child(id, Edge::Index(index)).ok_or(Error::Typed {
                        address: id,
                        field,
                        reason: "missing indexed address",
                    })?;
                    bind_slot(
                        &mut bound,
                        &mut bound_addresses,
                        owner,
                        Slot::Element { field, index },
                        child,
                    )?;
                    pending.push(Task::Array(owner, field, id, values, index + 1, raw));
                    value(
                        &mut pending,
                        owner,
                        field,
                        child,
                        values.get(index),
                        &raw[index],
                    )?;
                }
            }
        }
        if pending.len() > MAX_PENDING {
            return Err(Error::Pending);
        }
    }
    bound.transport_only = bound_addresses.iter().filter(|seen| !**seen).count();
    Ok(bound)
}

fn bind_slot<'ir>(
    bound: &mut BoundOccurrences<'ir>,
    seen: &mut [bool],
    owner: Owner<'ir>,
    slot: Slot,
    id: usize,
) -> Result<(), Error> {
    if bound.entries.insert((owner, slot), id).is_some() {
        return Err(Error::Typed {
            address: id,
            field: "",
            reason: "same borrowed slot bound twice",
        });
    }
    seen[id] = true;
    Ok(())
}

fn value<'ir, 'raw>(
    pending: &mut Vec<Task<'ir, 'raw>>,
    owner: Owner<'ir>,
    field: &'static str,
    id: usize,
    value: TypedValue<'ir>,
    raw: &'raw Value,
) -> Result<(), Error> {
    match value {
        TypedValue::Node(node) => pending.push(Task::Node(node, id, raw)),
        TypedValue::Null if raw.is_null() => {}
        TypedValue::Null => {
            return Err(Error::Typed {
                address: id,
                field,
                reason: "optional source/member null mismatch",
            });
        }
        TypedValue::Scalar(scalar) => {
            let same = match scalar {
                Scalar::Text(v) => raw.as_str() == Some(v),
                Scalar::Bool(v) => raw.as_bool() == Some(v),
                Scalar::Integer(v) => raw.as_u64() == Some(v),
            };
            if !same {
                return Err(Error::Typed {
                    address: id,
                    field,
                    reason: "scalar differs from borrowed typed owner",
                });
            }
        }
        TypedValue::Slice(values) => {
            let raw = raw.as_array().ok_or(Error::Typed {
                address: id,
                field,
                reason: "typed slice must be raw array",
            })?;
            if raw.len() != values.len() {
                return Err(Error::Typed {
                    address: id,
                    field,
                    reason: "typed slice length mismatch",
                });
            }
            pending.push(Task::Array(owner, field, id, values, 0, raw));
        }
    }
    Ok(())
}
