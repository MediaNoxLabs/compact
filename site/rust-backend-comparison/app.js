// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

const stages = {
  boundary: {
    kicker: "COMPILER BOUNDARY",
    number: "01 / 04",
    oracleTitle: "Shared TypeScript IR",
    oracleDescription:
      "The Rust pass reads Ltypescript, the same prepared IR consumed by the TypeScript backend. The compiler directly emits a Rust crate with --target rust.",
    oracleRoute: ["Compact", "Ltypescript", "Scheme emitter"],
    localTitle: "Pre-TypeScript boundary",
    localDescription:
      "The Rust IR pass starts at Lnodisclose, before TypeScript-specific preparation. A separate Rust process validates and renders its output.",
    localRoute: ["Compact", "Lnodisclose", "schema-6 IR"],
    insight: "Cleaner target separation, with a second process and a JSON compatibility boundary.",
  },
  model: {
    kicker: "DOMAIN MODEL",
    number: "02 / 04",
    oracleTitle: "Implicit Scheme context",
    oracleDescription:
      "Nanopass nodes, parameters, naming tables, and walkability predicates carry the information needed to emit Rust. Many Rust shapes are produced as strings.",
    oracleRoute: ["Nanopass nodes", "walker routes", "string fragments"],
    localTitle: "Closed, versioned contract model",
    localDescription:
      "Contract, Type, Expr, ConstructorStep, StateAction, and StateReturn are explicit serde variants. Unknown fields, schema versions, and many type mismatches fail early.",
    localRoute: ["Scheme lowering", "typed JSON", "Rust validation"],
    insight: "The boundary is inspectable and testable; renderer errors still need Compact source spans.",
  },
  emission: {
    kicker: "RUST EMISSION",
    number: "03 / 04",
    oracleTitle: "Buffered text + rustfmt",
    oracleDescription:
      "Nine included Scheme files select constructor, pure, impure, and streaming routes. They buffer text through out, guard known bad placeholders, then format Rust.",
    oracleRoute: ["walkable?", "out fragments", "rustfmt"],
    localTitle: "Rust syntax trees",
    localDescription:
      "The Rust renderer checks the closed IR, builds syn syntax with quote and proc-macro2, and formats the resulting file with prettyplease.",
    localRoute: ["IR variant", "syn nodes", "prettyplease"],
    insight: "Structured syntax removes text splicing; sizeable parse_quote templates still need review.",
  },
  runtime: {
    kicker: "RUNTIME HANDOFF",
    number: "04 / 04",
    oracleTitle: "Generated opcode chains",
    oracleDescription:
      "The generated Contract methods assemble OpProgramVerify/Gather instructions. Runtime builders, query helpers, and standard-library adapters execute them on ledger-8.",
    oracleRoute: ["Contract method", "OpProgram", "ledger VM"],
    localTitle: "Typed context operations",
    localDescription:
      "Generated ledger_contract functions call context and ADT methods. CellValue codecs and VM opcode construction sit in the native runtime.",
    localRoute: ["Circuit function", "context action", "ledger VM"],
    insight: "The generated surface is simpler; the runtime ledger adapter carries more policy and code.",
  },
};

const typeMappings = {
  uint: {
    source: "COMPACT / Uint<8>",
    category: "PRIMITIVE 01",
    oracle: "u8",
    oracleDetail:
      "The width maps to a native Rust integer; Compact's maximum is enforced by emitted operations and runtime behavior.",
    local: "BoundedUint<255>",
    localDetail:
      "The exact inclusive maximum is carried by the Rust type and checked at construction and decoding.",
    takeaway: "The local API makes the bound visible to Rust callers.",
  },
  bytes: {
    source: "COMPACT / Bytes<32>",
    category: "PRIMITIVE 02",
    oracle: "[u8; 32]",
    oracleDetail:
      "A native fixed array gives callers direct access to bytes; encoding rules are supplied through the runtime.",
    local: "FixedBytes<32>",
    localDetail:
      "A local newtype carries the fixed length and composes ledger FAB and field representation traits.",
    takeaway: "The wrapper makes Compact byte semantics an explicit Rust type.",
  },
  vector: {
    source: "COMPACT / Vector<3, Field>",
    category: "COMPOSITE 03",
    oracle: "[Fr; 3]",
    oracleDetail:
      "Rust's array length represents the vector length, while generated code handles the Compact representation.",
    local: "FixedVector<Fr, 3>",
    localDetail:
      "The runtime newtype adds the missing representation and decoding composition for generated structs and Cells.",
    takeaway: "Both retain length in the Rust type; the local wrapper centralizes encoding.",
  },
  opaque: {
    source: 'COMPACT / Opaque<"Uint8Array">',
    category: "OPAQUE 04",
    oracle: "Vec<u8>",
    oracleDetail:
      "The oracle uses a familiar dynamic byte vector with upstream representation implementations.",
    local: "OpaqueBytes",
    localDetail:
      "A dedicated runtime type carries Compact opaque-byte behavior and has explicit FAB and field mappings.",
    takeaway: "The local type distinguishes opaque bytes from ordinary byte collections.",
  },
};

function routeElement(values) {
  const fragment = document.createDocumentFragment();
  values.forEach((value, index) => {
    if (index > 0) {
      const arrow = document.createElement("i");
      arrow.setAttribute("aria-hidden", "true");
      arrow.textContent = "→";
      fragment.appendChild(arrow);
    }
    const label = document.createElement("span");
    label.textContent = value;
    fragment.appendChild(label);
  });
  return fragment;
}

function selectStage(name) {
  const stage = stages[name];
  if (!stage) return;
  document.querySelectorAll("[data-stage]").forEach((button) => {
    const selected = button.dataset.stage === name;
    button.classList.toggle("is-active", selected);
    button.setAttribute("aria-pressed", String(selected));
  });
  document.getElementById("stage-kicker").textContent = stage.kicker;
  document.getElementById("stage-number").textContent = stage.number;
  document.getElementById("oracle-stage-title").textContent = stage.oracleTitle;
  document.getElementById("oracle-stage-description").textContent = stage.oracleDescription;
  document.getElementById("local-stage-title").textContent = stage.localTitle;
  document.getElementById("local-stage-description").textContent = stage.localDescription;
  document.getElementById("oracle-stage-code").replaceChildren(routeElement(stage.oracleRoute));
  document.getElementById("local-stage-code").replaceChildren(routeElement(stage.localRoute));
  const insight = document.getElementById("stage-insight");
  const label = document.createElement("span");
  label.textContent = "THE TRADEOFF";
  insight.replaceChildren(label, document.createTextNode(stage.insight));
}

function selectType(name) {
  const mapping = typeMappings[name];
  if (!mapping) return;
  document.querySelectorAll("[data-type]").forEach((button) => {
    const selected = button.dataset.type === name;
    button.classList.toggle("is-active", selected);
    button.setAttribute("aria-pressed", String(selected));
  });
  document.getElementById("type-source").textContent = mapping.source;
  document.getElementById("type-category").textContent = mapping.category;
  document.getElementById("oracle-type").textContent = mapping.oracle;
  document.getElementById("oracle-type-detail").textContent = mapping.oracleDetail;
  document.getElementById("local-type").textContent = mapping.local;
  document.getElementById("local-type-detail").textContent = mapping.localDetail;
  document.getElementById("type-takeaway").textContent = mapping.takeaway;
}

document.querySelectorAll("[data-stage]").forEach((button) => {
  button.addEventListener("click", () => selectStage(button.dataset.stage));
});
document.querySelectorAll("[data-type]").forEach((button) => {
  button.addEventListener("click", () => selectType(button.dataset.type));
});
