//! First-insertion provenance, not structure matching against output clones.
use super::*;
use crate::{
    ir::{StructField, Type},
    source_diagnostics::{Owner, RefusalObserver, RefusalSite},
    type_declarations,
};
use std::{cell::RefCell, collections::BTreeMap};

fn member_path(parameter: usize) -> Vec<Value> {
    vec![
        json!({"field":"circuits"}),
        json!({"index":0}),
        json!({"field":"parameters"}),
        json!({"index":parameter}),
        json!({"field":"ty"}),
        json!({"field":"fields"}),
        json!({"index":0}),
        json!({"field":"ty"}),
        json!({"field":"max"}),
    ]
}

#[test]
fn repeated_equal_definitions_select_the_first_original_member() {
    let ty = json!({"kind":"struct","name":"Shared","fields":[{"name":"amount","ty":{"kind":"unsigned","max":"bad"}}]});
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] = json!([{"name":"first","ty":ty},{"name":"second","ty":ty}]);
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = raw(&ir, &seed.source);
    let first = address(&metadata, &member_path(0));
    let second = address(&metadata, &member_path(1));
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    let failed = failure(&sandbox);
    assert_eq!(diagnostic(&failed).address_id, first);
    assert_ne!(first, second);
    assert_eq!(diagnostic(&failed).selected_slot, "max");
}

#[test]
fn nested_definition_member_keeps_original_annotation_identity() {
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] = json!([{"name":"value","ty":{"kind":"struct","name":"Outer","fields":[{"name":"nested","ty":{"kind":"struct","name":"Inner","fields":[{"name":"amount","ty":{"kind":"unsigned","max":"bad"}}]}}]}}]);
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = raw(&ir, &seed.source);
    let mut path = member_path(0);
    path.pop();
    path.extend([
        json!({"field":"fields"}),
        json!({"index":0}),
        json!({"field":"ty"}),
        json!({"field":"max"}),
    ]);
    let expected = address(&metadata, &path);
    let sandbox = Sandbox::new(&ir, Some(&serde_json::to_vec(&metadata).unwrap()), None, 0);
    assert_eq!(diagnostic(&failure(&sandbox)).address_id, expected);
}

#[test]
fn successful_named_emission_remains_byte_equal_with_two_equal_sources() {
    let ty =
        json!({"kind":"struct","name":"Shared","fields":[{"name":"amount","ty":{"kind":"field"}}]});
    let mut ir = pure(field());
    ir["circuits"][0]["parameters"] = json!([{"name":"first","ty":ty},{"name":"second","ty":ty}]);
    let expected = crate::render(&serde_json::from_value(ir.clone()).unwrap()).unwrap();
    let seed = Sandbox::new(&ir, None, None, 0);
    let metadata = fixture::metadata(&ir, &seed.source, false);
    let sandbox = Sandbox::new(&ir, Some(&metadata), None, 0);
    assert_eq!(
        sandbox
            .invoke()
            .render(RenderOptions::default())
            .unwrap()
            .rendered()
            .source,
        expected
    );
}

struct Recorder {
    sites: RefCell<Vec<(bool, &'static str)>>,
}
impl RefusalObserver for Recorder {
    fn refused(&self, site: RefusalSite<'_>) {
        self.sites.borrow_mut().push((
            site.owner.is_some(),
            match site.slot {
                crate::source_diagnostics::Slot::Field(name) => name,
                _ => "node",
            },
        ));
    }
}

#[test]
fn synthesized_definition_has_no_manufactured_original_origin() {
    let mut structs = BTreeMap::new();
    structs.insert(
        "Synthesized".into(),
        vec![StructField {
            name: "amount".into(),
            ty: Type::Unsigned { max: "bad".into() },
        }],
    );
    let observer = Recorder {
        sites: RefCell::new(Vec::new()),
    };
    let outcome = type_declarations::render(
        &structs,
        &BTreeMap::new(),
        &[],
        false,
        false,
        &type_declarations::DefinitionOrigins::new(),
        Some(&observer),
    );
    assert!(outcome.is_err());
    assert_eq!(*observer.sites.borrow(), vec![(false, "node")]);
}

#[test]
fn provenance_is_recorded_only_at_the_original_successful_first_insertion() {
    let first = Type::Struct {
        name: "Shared".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let repeated = first.clone();
    let mut structs = BTreeMap::new();
    let mut enums = BTreeMap::new();
    let mut originals = type_declarations::DefinitionOrigins::new();
    crate::collect_named_types_with_origins(&first, &mut structs, &mut enums, Some(&mut originals))
        .unwrap();
    crate::collect_named_types_with_origins(
        &repeated,
        &mut structs,
        &mut enums,
        Some(&mut originals),
    )
    .unwrap();
    assert!(std::ptr::eq(originals["Shared"], &first));
    assert!(!std::ptr::eq(originals["Shared"], &repeated));
    let synthetic = Type::Struct {
        name: "Synthesized".into(),
        fields: vec![],
    };
    crate::collect_named_types_with_origins(&synthetic, &mut structs, &mut enums, None).unwrap();
    crate::collect_named_types_with_origins(
        &synthetic,
        &mut structs,
        &mut enums,
        Some(&mut originals),
    )
    .unwrap();
    assert!(
        !originals.contains_key("Synthesized"),
        "later references cannot supply a fake first origin"
    );
    // Owner identity remains pointer/category based; no semantic Eq lookup.
    assert_ne!(Owner::Type(&first), Owner::Type(&repeated));
}
