//! Order-independent complete edge coverage. No lexical ordinal guarantee.
use crate::origins::binding::{
    Error, MAX_PENDING,
    addresses::{AddressAccess, Edge, Graph},
};
use serde_json::Value;

enum Task<'a> {
    Node(usize, &'a Value),
    Object(usize, serde_json::map::Iter<'a>),
    Array(usize, usize, std::slice::Iter<'a, Value>),
}

pub(crate) fn verify<A: AddressAccess>(raw: &Value, graph: &Graph<'_, A>) -> Result<(), Error> {
    let mut pending = vec![Task::Node(0, raw)];
    let mut seen = vec![false; graph.source.len()];
    while let Some(task) = pending.pop() {
        match task {
            Task::Node(id, value) => {
                if seen[id] {
                    return Err(Error::Shape {
                        address: id,
                        reason: "repeated occurrence",
                    });
                }
                seen[id] = true;
                let actual_kind = value
                    .as_object()
                    .and_then(|map| map.get("kind"))
                    .and_then(Value::as_str);
                if actual_kind != graph.source.address(id).kind {
                    return Err(Error::Shape {
                        address: id,
                        reason: "kind differs from actual raw node",
                    });
                }
                let count = match value {
                    Value::Object(map) => map.len(),
                    Value::Array(array) => array.len(),
                    _ => 0,
                };
                if count != graph.count(id) {
                    return Err(Error::Shape {
                        address: id,
                        reason: "child coverage",
                    });
                }
                match value {
                    Value::Object(map) => pending.push(Task::Object(id, map.iter())),
                    Value::Array(array) => pending.push(Task::Array(id, 0, array.iter())),
                    _ => {}
                }
            }
            Task::Object(id, mut children) => {
                if let Some((field, value)) = children.next() {
                    let child = graph.child(id, Edge::Field(field)).ok_or(Error::Shape {
                        address: id,
                        reason: "missing raw field edge",
                    })?;
                    pending.push(Task::Object(id, children));
                    pending.push(Task::Node(child, value));
                }
            }
            Task::Array(id, index, mut children) => {
                if let Some(value) = children.next() {
                    let child = graph.child(id, Edge::Index(index)).ok_or(Error::Shape {
                        address: id,
                        reason: "missing raw array edge",
                    })?;
                    pending.push(Task::Array(id, index + 1, children));
                    pending.push(Task::Node(child, value));
                }
            }
        }
        if pending.len() > MAX_PENDING {
            return Err(Error::Pending);
        }
    }
    if seen.iter().any(|value| !value) {
        return Err(Error::Addresses("unreachable raw address"));
    }
    Ok(())
}
