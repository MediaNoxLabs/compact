//! Private lossless tuples; the validated internal domain retains named fields.
use crate::origins::transport::model::{
    self, Category, Edge, Endpoint, Extent, File, Label, Nullable, Origin, Table,
};
use crate::origins::transport::{Error, wire_shape};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Document {
    schema_version: u64,
    #[serde(deserialize_with = "wire_shape::string_only")]
    coordinate_system: model::CoordinateSystem,
    end_exclusive: bool,
    #[serde(default, deserialize_with = "wire_shape::optional_object_table")]
    files: Option<Table<File>>,
    #[serde(default, deserialize_with = "wire_shape::optional_table")]
    spans: Option<Table<Span>>,
    #[serde(default, deserialize_with = "wire_shape::optional_table")]
    origins: Option<Table<OriginReference>>,
    #[serde(default, deserialize_with = "wire_shape::optional_table")]
    addresses: Option<Table<Address>>,
    #[serde(default, deserialize_with = "wire_shape::optional_map")]
    error: Option<Refusal>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Span {
    file_id: u64,
    start: Endpoint,
    end: Endpoint,
    extent: Extent,
}

#[derive(Deserialize)]
struct SpanTuple(
    u64,
    u64,
    u64,
    u64,
    u64,
    u64,
    u64,
    #[serde(deserialize_with = "wire_shape::map_only")] Extent,
);

impl<'de> Deserialize<'de> for Span {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let SpanTuple(
            file_id,
            start_line,
            start_column,
            start_offset,
            end_line,
            end_column,
            end_offset,
            extent,
        ) = SpanTuple::deserialize(deserializer)?;
        Ok(Self {
            file_id,
            start: Endpoint {
                line: start_line,
                column: start_column,
                offset: start_offset,
            },
            end: Endpoint {
                line: end_line,
                column: end_column,
                offset: end_offset,
            },
            extent,
        })
    }
}

#[derive(Debug)]
struct OriginReference {
    span_id: u64,
    label: Label,
    category: Category,
}

#[derive(Deserialize)]
struct OriginTuple(
    u64,
    #[serde(deserialize_with = "wire_shape::string_only")] Label,
    #[serde(deserialize_with = "wire_shape::string_only")] Category,
);

impl<'de> Deserialize<'de> for OriginReference {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let OriginTuple(span_id, label, category) = OriginTuple::deserialize(deserializer)?;
        Ok(Self {
            span_id,
            label,
            category,
        })
    }
}

#[derive(Debug)]
struct Address {
    parent: Nullable<u64>,
    edge: Edge,
    kind: Nullable<String>,
    origin: Nullable<u64>,
}

#[derive(Deserialize)]
struct AddressTuple(
    Nullable<u64>,
    #[serde(deserialize_with = "wire_shape::map_only")] Edge,
    Nullable<String>,
    Nullable<u64>,
);

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let AddressTuple(parent, edge, kind, origin) = AddressTuple::deserialize(deserializer)?;
        Ok(Self {
            parent,
            edge,
            kind,
            origin,
        })
    }
}

#[derive(Debug, Deserialize)]
enum NumericResource {
    #[serde(rename = "origin_numeric_width")]
    Width,
}
#[derive(Debug, Deserialize)]
enum NumericObservation {
    #[serde(rename = "outside_u64")]
    OutsideU64,
}
#[derive(Debug, Deserialize)]
enum NumericLimit {
    #[serde(rename = "18446744073709551615")]
    U64Max,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericRefusal {
    #[serde(deserialize_with = "wire_shape::string_only")]
    resource: NumericResource,
    #[serde(deserialize_with = "wire_shape::string_only")]
    observed: NumericObservation,
    #[serde(deserialize_with = "wire_shape::string_only")]
    limit: NumericLimit,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Refusal {
    Numeric(NumericRefusal),
    Count(model::ProducerRefusal),
}

fn invalid(index: usize, reason: &'static str) -> Error {
    Error::Structure {
        table: "spans",
        index,
        reason,
    }
}

/// Expand the private wire into the old complete named origin facts. Bounds and
/// references precede expansion allocation; no borrowed/self-referential index.
pub(crate) fn expand(document: Document) -> Result<model::Document, Error> {
    if document.schema_version != 3 || !document.end_exclusive {
        return Err(Error::Header);
    }
    if let Some(error) = document.error {
        if document.files.is_some()
            || document.spans.is_some()
            || document.origins.is_some()
            || document.addresses.is_some()
        {
            return Err(Error::MixedProducerRefusal);
        }
        return match error {
            Refusal::Numeric(marker) => {
                let _bounded_marker = (marker.resource, marker.observed, marker.limit);
                Err(Error::ProducerNumericWidthRefusal)
            }
            Refusal::Count(error) => {
                if error.observed <= error.limit {
                    return Err(Error::MixedProducerRefusal);
                }
                Err(Error::ProducerRefusal {
                    resource: error.resource,
                    observed: error.observed,
                    limit: error.limit,
                })
            }
        };
    }
    let files = document.files.ok_or(Error::MissingTables)?;
    let spans = document.spans.ok_or(Error::MissingTables)?;
    let references = document.origins.ok_or(Error::MissingTables)?;
    let addresses = document.addresses.ok_or(Error::MissingTables)?;
    if spans.len() > references.len() || references.len() > addresses.len() {
        return Err(invalid(0, "span/origin/occurrence count ordering invalid"));
    }
    let mut unique = HashSet::new();
    for (index, span) in spans.0.iter().enumerate() {
        if !unique.insert(span) {
            return Err(invalid(index, "duplicate complete span"));
        }
    }
    let mut next_span = 0u64;
    let mut origins = Vec::with_capacity(references.len());
    for (index, reference) in references.0.into_iter().enumerate() {
        let span = spans
            .0
            .get(
                usize::try_from(reference.span_id)
                    .map_err(|_| invalid(index, "span reference out of range"))?,
            )
            .ok_or(invalid(index, "span reference out of range"))?;
        // First-occurrence intern order is deterministic and every span is used.
        if reference.span_id == next_span {
            next_span += 1;
        } else if reference.span_id > next_span {
            return Err(invalid(index, "span first-reference order invalid"));
        }
        origins.push(Origin {
            file_id: span.file_id,
            start: span.start.clone(),
            end: span.end.clone(),
            label: reference.label,
            category: reference.category,
            extent: span.extent.clone(),
        });
    }
    if next_span != spans.len() as u64 {
        return Err(invalid(0, "unreferenced span"));
    }
    let addresses = addresses
        .0
        .into_iter()
        .enumerate()
        .map(|(id, address)| model::Address {
            id: id as u64,
            parent: address.parent,
            edge: address.edge,
            kind: address.kind,
            origin: address.origin,
        })
        .collect();
    Ok(model::Document {
        schema_version: 3,
        coordinate_system: document.coordinate_system,
        end_exclusive: true,
        files: Some(files),
        origins: Some(Table(origins)),
        addresses: Some(Table(addresses)),
        error: None,
    })
}
