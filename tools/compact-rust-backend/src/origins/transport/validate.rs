use crate::origins::transport::model::{Document, Edge, Extent, Tables};
use crate::origins::transport::{Error, MAX_PATH_CHARACTERS, MAX_PENDING};
use std::collections::{HashMap, HashSet};

fn invalid(table: &'static str, index: usize, reason: &'static str) -> Error {
    Error::Structure {
        table,
        index,
        reason,
    }
}

pub(crate) fn validate(document: Document) -> Result<Tables, Error> {
    if document.schema_version != 3 || !document.end_exclusive {
        return Err(Error::Header);
    }
    // CoordinateSystem has one closed variant, already checked by deserialization.
    let _coordinates = document.coordinate_system;
    if let Some(error) = document.error {
        if document.files.is_some()
            || document.origins.is_some()
            || document.addresses.is_some()
            || error.observed <= error.limit
        {
            return Err(Error::MixedProducerRefusal);
        }
        return Err(Error::ProducerRefusal {
            resource: error.resource,
            observed: error.observed,
            limit: error.limit,
        });
    }
    let tables = Tables {
        files: document.files.ok_or(Error::MissingTables)?,
        origins: document.origins.ok_or(Error::MissingTables)?,
        addresses: document.addresses.ok_or(Error::MissingTables)?,
    };
    let mut paths = HashSet::new();
    let mut path_characters = 0usize;
    for (index, file) in tables.files.0.iter().enumerate() {
        if file.id != index as u64 || file.path.is_empty() || !paths.insert(&file.path) {
            return Err(invalid(
                "files",
                index,
                "non-dense id, empty path or duplicate path",
            ));
        }
        path_characters = path_characters
            .checked_add(file.path.chars().count())
            .ok_or(Error::PathCharactersExceeded)?;
        if path_characters > MAX_PATH_CHARACTERS {
            return Err(Error::PathCharactersExceeded);
        }
    }
    if tables.addresses.0.is_empty() || tables.origins.len() > tables.addresses.len() {
        return Err(invalid(
            "addresses",
            0,
            "root missing or more origins than occurrences",
        ));
    }
    let mut unique_origins = HashSet::new();
    for (index, origin) in tables.origins.0.iter().enumerate() {
        if origin.file_id >= tables.files.len() as u64 {
            return Err(invalid("origins", index, "file reference out of range"));
        }
        if origin.start.line == 0
            || origin.start.column == 0
            || origin.end.line == 0
            || origin.end.column == 0
            || origin.start.offset > origin.end.offset
            || (origin.start.line, origin.start.column) > (origin.end.line, origin.end.column)
        {
            return Err(invalid("origins", index, "invalid endpoint ordering"));
        }
        if matches!(
            origin.extent,
            Extent::CompilerContext {
                attachable: true,
                ..
            }
        ) {
            return Err(invalid(
                "origins",
                index,
                "compiler context cannot be attachable",
            ));
        }
        if !unique_origins.insert(origin) {
            return Err(invalid("origins", index, "duplicate origin record"));
        }
    }
    // No recursive walk or self-referential index. Earlier-parent plus active
    // ancestor checks enforce a connected dense depth-first occurrence tree.
    let mut ancestors = Vec::new();
    let mut sibling_edges = HashSet::new();
    let mut child_shapes = HashMap::new();
    for (index, address) in tables.addresses.0.iter().enumerate() {
        if address.id != index as u64 {
            return Err(invalid("addresses", index, "non-dense id"));
        }
        if address
            .origin
            .0
            .is_some_and(|id| id >= tables.origins.len() as u64)
        {
            return Err(invalid("addresses", index, "origin reference out of range"));
        }
        if index == 0 {
            if address.parent.0.is_some()
                || !matches!(address.edge, Edge::Root(ref edge) if edge.root)
            {
                return Err(invalid("addresses", index, "root parent or edge invalid"));
            }
        } else {
            let parent = address
                .parent
                .0
                .ok_or(invalid("addresses", index, "extra root"))?;
            if parent >= address.id || matches!(address.edge, Edge::Root(_)) {
                return Err(invalid(
                    "addresses",
                    index,
                    "forward parent or non-root root edge",
                ));
            }
            while ancestors.last().is_some_and(|id| *id != parent) {
                ancestors.pop();
            }
            if ancestors.last() != Some(&parent) {
                return Err(invalid(
                    "addresses",
                    index,
                    "parent is outside active preorder ancestry",
                ));
            }
            if !sibling_edges.insert((parent, &address.edge)) {
                return Err(invalid("addresses", index, "duplicate sibling edge"));
            }
            // This checks only the transport tree, not the actual IR container.
            // Producer vector edges are dense and ordered; object/vector child
            // shapes cannot be mixed under one parent.
            match &address.edge {
                Edge::Field(_) => {
                    if child_shapes
                        .insert(parent, None)
                        .is_some_and(|shape| shape.is_some())
                    {
                        return Err(invalid(
                            "addresses",
                            index,
                            "mixed object and vector child edges",
                        ));
                    }
                }
                Edge::Index(edge) => {
                    let expected = child_shapes.entry(parent).or_insert(Some(0u64));
                    if *expected != Some(edge.index) {
                        return Err(invalid(
                            "addresses",
                            index,
                            "non-dense vector index or mixed child edges",
                        ));
                    }
                    *expected = edge.index.checked_add(1);
                }
                Edge::Root(_) => return Err(invalid("addresses", index, "non-root root edge")),
            }
        }
        if ancestors.len() == MAX_PENDING {
            return Err(Error::PendingExceeded);
        }
        ancestors.push(address.id);
        // A tag is metadata, not evidence that this address denotes a typed Expr.
        let _unbound_kind = &address.kind;
    }
    Ok(tables)
}
