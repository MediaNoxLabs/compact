window.STORYBOOK_DATA = {
  examples: {
    state: {
      sourceLabel: "circuit set",
      rustLabel: "native path",
      source: `circuit set(v: Field): [] {
  assert(in_state(STATE.unset),
    "set: attempted to overwrite recorded value");
  const sk = private$secret_key();
  authority = public_key(sk);
  value = disclose(v);
  state = STATE.set;
}`,
      rust: `let prior = in_state(context, STATE::unset)?;
context = prior.context;
total_cost += prior.gas_cost;
if !prior.result {
    return Err(CompactError::AssertionFailed(message));
}
let step = ledger_slots::value.write(context, v)?;
context = step.context;
total_cost += step.gas_cost;`,
      note: "The source assertion becomes a typed runtime error; the slot owns the ledger operation; the generated body preserves source order and accumulates actual query cost."
    },
    recording: {
      sourceLabel: "circuit get",
      rustLabel: "recorded path",
      source: `circuit get(): Maybe<Field> {
  return in_state(STATE.set)
    ? some<Field>(value)
    : none<Field>();
}`,
      rust: `let frame = RecordingFrame::new(context);
let (frame, state) = ledger_slots::state
    .record_read(frame)?;
let (frame, observed) = if state == STATE::set {
    let (frame, value) = ledger_slots::value
        .record_read(frame)?;
    (frame, some(value)?)
} else { (frame, none()?) };
Ok(frame.finish(observed))`,
      note: "The recorded path evaluates the same branch while retaining ordered Verify operations. The result pairs native execution with PublicTrace; it is not yet a proof or transaction."
    },
    types: {
      sourceLabel: "source types",
      rustLabel: "mechanical derives",
      source: `enum STATE { unset, set }

ledger authority: Bytes<32>;
export ledger value: Field;
ledger state: STATE;`,
      rust: `#[derive(Clone, Copy, Debug, PartialEq, Eq,
         runtime::CompactCellValue,
         runtime::CompactEnum)]
pub enum STATE { unset, set }

pub const state: CellSlot<STATE> =
    CellSlot::new(&[2]);`,
      note: "The compiler selects the Rust type and physical declaration path. Derives add checked representation glue; CellSlot owns typed access to the ledger-8 operation."
    },
    consumer: {
      sourceLabel: "three source capabilities",
      rustLabel: "consumer calls",
      source: `// Same Compact entry point;
// three different evidence stages:
// 1. execute
// 2. execute + record
// 3. bind observed state + artifacts`,
      rust: `let native = contract.set(context, value)?;

let recorded = contract.recording()
    .set(context, value)?;

let call = contract.recording()
    .set_call(&observed, private, value)?;
let prototype = call.prepare(verifier, randomness)?;`,
      note: "A compact facade does not flatten the evidence ladder. Native execution, complete recording and observed artifact-bound preparation remain separate capabilities and failure surfaces."
    }
  },
  stages: {
    frontend: { kicker: "SOURCE AUTHORITY", owner: "Compact parser, type checker and shared analysis", crosses: "Typed declarations, analyzed expressions and safe source-owner metadata.", limit: "Frontend acceptance is not native behavior, recording support or proof applicability." },
    ir: { kicker: "CLOSED COMPILER BOUNDARY", owner: "Private Rust-target IR and schema validation", crosses: "Explicit types, declarations, expressions, capabilities and structural paths.", limit: "The private schema is not a public stable API and arbitrary nested source ranges are not complete." },
    semantics: { kicker: "MEANING BEFORE SYNTAX", owner: "Native lowering plus checked recording-effect rules", crosses: "Typed facts, ordered effects, scopes, resource requirements and replay invariants.", limit: "An admitted native expression does not automatically gain a recordable or provable form." },
    render: { kicker: "MECHANICAL TARGET CONSTRUCTION", owner: "syn, quote, proc-macro2 and prettyplease", crosses: "Validated Rust items, expressions, types and diagnostics.", limit: "Pretty output is not evidence of semantic parity; syntax construction should not decide source meaning." },
    runtime: { kicker: "LEDGER-8 AUTHORITY", owner: "Generated crate, Compact runtime, official ledger primitives and ZK stack", crosses: "Values, contexts, recorded plans, prepared transactions, proofs and application receipts.", limit: "Proof verification is distinct from transaction application, and neither implies live-network finality." }
  },
  comparisons: {
    boundary: [
      ["TypeScript / JS", "Established preparation and runtime path; frontend/source-map behavior is native to the reference ecosystem.", "REFERENCE PATH"],
      ["Rust V1", "Chez consumed a TypeScript-oriented representation and emitted Rust source directly.", "DESTINATION ORACLE"],
      ["Rust-AST", "Chez emits a closed private typed IR before TypeScript-specific lowering; Rust validates the schema.", "FAIL-CLOSED"],
      ["AST + algebra", "The same IR feeds shared checked semantic folds and explicit effect planning.", "COMPOSITIONAL"]
    ],
    syntax: [
      ["TypeScript / JS", "The established generator and TypeScript toolchain own target syntax.", "ESTABLISHED"],
      ["Rust V1", "Scheme walkers splice Rust strings while also deciding semantic admission.", "TEXT EMISSION"],
      ["Rust-AST", "Rust syntax is built with typed AST libraries and formatted mechanically.", "STRUCTURAL"],
      ["AST + algebra", "Effect and execution meaning is settled before the existing AST renderer runs.", "SYNTAX-FREE RULES"]
    ],
    runtime: [
      ["TypeScript / JS", "JS/WASM providers and ledger APIs define the established integration surface.", "REFERENCE"],
      ["Rust V1", "A native runtime and VM-oriented generated code prove feasibility.", "PROTOTYPE"],
      ["Rust-AST", "Typed runtime layers reuse official ledger-8 primitives and generated Cargo consumers.", "LEDGER-8"],
      ["AST + algebra", "Authority stays unchanged while execution and result roles become clearer.", "SAME AUTHORITY"]
    ],
    recording: [
      ["TypeScript / JS", "Established semantics, but a different generated and consumer surface.", "REFERENCE"],
      ["Rust V1", "Recording logic follows source walkers and emitted target arrangements.", "COUPLED"],
      ["Rust-AST", "Typed gaps and specialized whole-program profiles expand support safely but accumulate.", "BOUNDED PROFILES"],
      ["AST + algebra", "Read, witness, assert, write and related operations compose into a checked plan.", "EFFECT PLAN"]
    ],
    evidence: [
      ["TypeScript / JS", "Reference behavior and integration corpus; not automatically the denominator for all Rust claims.", "REFERENCE CORPUS"],
      ["Rust V1", "Retained as an output and behavior oracle at pinned revisions.", "ORACLE"],
      ["Rust-AST", "Closed milestone matrices separate compile, native, record, replay, proof and apply.", "SCOPED CLOSE"],
      ["AST + algebra", "Per-family receipts plus 974-case historical validation; final frozen EA13 matrix remains open.", "ACTIVE PROTOTYPE"]
    ]
  },
  evidence: {
    inventory: ["01", "Source inventory", "Names inputs, exports and classified capabilities.", "Execution, recording, proof or specification coverage."],
    compile: ["02", "Compiler acceptance", "Shows a source reaches the checked target—or is deliberately refused.", "Correct runtime behavior or compatibility."],
    native: ["03", "Native behavior", "Compares results, errors, state and witness order for named scenarios.", "Recording availability, proof support or universal parity."],
    record: ["04", "Recording", "Establishes ordered operations, resource accounting and admitted paths.", "Replay stability or proof generation."],
    replay: ["05", "Replay", "Shows a recorded plan reproduces checked effects under named invariants.", "That a prover accepts the artifact set."],
    proof: ["06", "Prove / verify", "Binds exact source, IR, Rust, keys, runtime and policy into a proof receipt.", "Ledger application or network finality."],
    apply: ["07", "Ledger application", "Shows a verified transaction applies under the pinned ledger policy.", "Production adoption, live-network observation or ledger-9 readiness."]
  },
  decisions: [
    ["01", "Move meaning ahead of Rust text", "String emission coupled semantic admission to formatting. A private typed IR made unsupported constructs fail before target construction and made the Rust boundary inspectable."],
    ["02", "Treat parity as bidirectional", "Rust exposed malformed source widths and Merkle underflow; TypeScript exposed Rust hash-to-curve, Schnorr and decoder defects. Neither implementation was declared the universal winner."],
    ["03", "Separate evidence stages", "A source count had been easy to overread. The close now distinguishes inventory, compile, native, record, replay, prove/verify, apply and live observation."],
    ["04", "Replace shapes with algebra", "Specialized recognizers delivered safe breadth, but every combination wanted another recognizer. Typed operations and a shared fold make the reuse unit semantic rather than program-shaped."],
    ["05", "Preserve source-bound trust", "Locations, structural paths and artifact identities make refusals and proof reuse auditable. Nested expression ranges and transient provenance remain explicit follow-up work."],
    ["06", "Keep ledger-9 outside the close", "Future streams can supply requirements, not evidence for the ledger-8 line. Passport ACC and foreign-curve breadth were removed instead of being implied by a broad application label."]
  ]
};
