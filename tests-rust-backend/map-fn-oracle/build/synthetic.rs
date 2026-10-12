// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit synthetic complete-IR laws; the genuine source and its root stay unchanged.
use serde_json::{Value, json};
fn field() -> Value {
    json!({"kind":"field"})
}
fn vector(element: Value, length: usize) -> Value {
    json!({"kind":"vector","element":element,"length":length})
}
fn param(name: &str) -> Value {
    json!({"kind":"parameter","name":name})
}
fn formal(name: &str, ty: Value) -> Value {
    json!({"name":name,"ty":ty})
}
fn number(value: u64) -> Value {
    json!({"kind":"field_literal","value":value.to_string()})
}
fn tuple(values: &[u64]) -> Value {
    json!({"kind":"tuple","elements":values.iter().map(|v|number(*v)).collect::<Vec<_>>()})
}
fn add(left: Value, right: Value) -> Value {
    json!({"kind":"add","left":left,"right":right})
}
fn call(name: &str, arguments: Vec<Value>) -> Value {
    json!({"kind":"call","name":name,"arguments":arguments})
}
fn map(source: Value, body: Value, result: Value, length: usize) -> Value {
    json!({"kind":"vector_map","parameter":formal("x",field()),"source":source,"body":body,"result":result,"length":length})
}
fn product(name: &str, fields: Vec<(&str, Value)>) -> Value {
    json!({"kind":"struct","name":name,"fields":fields.into_iter().map(|(name,ty)|json!({"name":name,"ty":ty})).collect::<Vec<_>>()})
}
fn literal(ty: Value, fields: Vec<Value>) -> Value {
    json!({"kind":"struct_literal","ty":ty,"fields":fields})
}
fn assertion(condition: Value, message: &str) -> Value {
    json!({"kind":"assert","condition":condition,"message":message})
}
fn sequence(steps: Vec<Value>, value: Value) -> Value {
    json!({"kind":"sequence","steps":steps,"value":value})
}
fn root(name: &str, parameters: Vec<Value>, result: Value, value: Value) -> Value {
    json!({"name":name,"parameters":parameters,"actions":[],"result":result,
        "return_value":{"kind":"expression","value":value},
        "source":{"file":"synthetic-vector-map-laws.ir","line":1,"column":1}})
}
pub fn contract(mut ir: Value) -> Value {
    let item = product(
        "MappedItem",
        vec![
            ("sum", field()),
            ("pair", json!({"kind":"tuple","elements":[field(),field()]})),
        ],
    );
    let batch = product(
        "ProductBatch",
        vec![("values", vector(item.clone(), 3)), ("outer", field())],
    );
    let mapped = map(
        param("values"),
        literal(
            item.clone(),
            vec![
                add(param("x"), param("offset")),
                json!({"kind":"tuple","elements":[param("x"),param("offset")]}),
            ],
        ),
        item,
        3,
    );
    let products = root(
        "products",
        vec![
            formal("values", vector(field(), 3)),
            formal("offset", field()),
            formal("x", field()),
        ],
        batch.clone(),
        literal(batch, vec![mapped, param("x")]),
    );

    // The inner x shadows the outer map formal; captured saves that outer value.
    let inner = map(
        tuple(&[3, 4]),
        add(add(param("x"), param("captured")), param("offset")),
        field(),
        2,
    );
    let outer_body = json!({"kind":"let","bindings":[{"name":"captured","ty":field(),"value":param("x")}],"body":inner});
    let matrix = map(tuple(&[1, 2]), outer_body, vector(field(), 2), 2);
    let nested_type = product(
        "NestedBatch",
        vec![
            ("matrix", vector(vector(field(), 2), 2)),
            ("outer", field()),
        ],
    );
    let nested = root(
        "nested",
        vec![formal("offset", field()), formal("x", field())],
        nested_type.clone(),
        literal(nested_type, vec![matrix, param("x")]),
    );

    let body = call("checked_item", vec![param("x")]);
    let empty = root(
        "empty",
        vec![],
        vector(field(), 0),
        map(
            json!({"kind":"vector","element":field(),"elements":[]}),
            body.clone(),
            field(),
            0,
        ),
    );
    let source = call("checked_source", vec![param("values"), param("source_ok")]);
    let fallible = root(
        "fallible",
        vec![
            formal("values", vector(field(), 3)),
            formal("source_ok", json!({"kind":"boolean"})),
        ],
        vector(field(), 3),
        map(source, body, field(), 3),
    );
    let mut witnessed = root(
        "witnessed",
        vec![],
        vector(field(), 3),
        json!({"kind":"unit"}),
    );
    // Capture the witness-bound value outside the pure mapping body.
    witnessed["return_value"]["value"] = json!({"kind":"let","bindings":[
        {"name":"offset","ty":field(),"value":{"kind":"witness_call","name":"offset","arguments":[]}}
    ],"body":map(tuple(&[1,2,3]),add(param("x"),param("offset")),field(),3)});

    ir["witnesses"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"offset","parameters":[],"result":field()}));
    ir["circuits"].as_array_mut().unwrap().extend([
        json!({"name":"checked_source","internal":true,"parameters":[formal("values",vector(field(),3)),formal("source_ok",json!({"kind":"boolean"}))],"result":vector(field(),3),
            "body":sequence(vec![assertion(param("source_ok"),"source refused")],param("values"))}),
        json!({"name":"checked_item","internal":true,"parameters":[formal("value",field())],"result":field(),
            "body":sequence(vec![
                assertion(json!({"kind":"not_equal","left":param("value"),"right":number(0)}),"zero item"),
                assertion(json!({"kind":"not_equal","left":param("value"),"right":number(2)}),"two item")
            ],add(param("value"),number(10)))})
    ]);
    ir["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .extend([products, nested, empty, fallible, witnessed]);
    ir
}
