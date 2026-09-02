;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0
;;; Licensed under the Apache License, Version 2.0 (the "License");
;;; you may not use this file except in compliance with the License.
;;; You may obtain a copy of the License at
;;;
;;;  	http://www.apache.org/licenses/LICENSE-2.0
;;;
;;; Unless required by applicable law or agreed to in writing, software
;;; distributed under the License is distributed on an "AS IS" BASIS,
;;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;;; See the License for the specific language governing permissions and
;;; limitations under the License.

;;; Part of the former single-file `rust-passes-emit.ss` (3,397 lines),
;;; split by capability so each piece is reviewable on its own. The split
;;; is textual: every one of these files is `include`d into the same
;;; `(definitions ...)` block in rust-passes.ss, where internal defines are
;;; mutually recursive, so grouping carries no ordering constraint and no
;;; behavioural change. Byte parity over the fixture corpus is what proves
;;; that.
;;;
;;; This file: the ledger view, decoders, defaults and the crate manifest.

      ;; pl-array->public-bindings: flatten a `public-ledger-array` IR node
      ;; into a list of `public-binding`s. Ported from
      ;; typescript-passes.ss::pl-array->public-bindings — nested arrays are
      ;; walked recursively; leaves (`public-binding`) accumulate.
      (define (pl-array->public-bindings pl-array)
        (let f ([pl-array pl-array] [pb* '()])
          (nanopass-case (Ltypescript Public-Ledger-Array) pl-array
            [(public-ledger-array ,pl-array-elt* ...)
             (fold-right
               (lambda (pl-array-elt pb*)
                 (nanopass-case (Ltypescript Public-Ledger-Array-Element) pl-array-elt
                   [,pl-array (f pl-array pb*)]
                   [,public-binding (cons public-binding pb*)]))
               pb*
               pl-array-elt*)])))

      ;; exported-public-binding?: #t if the binding's field-name id is
      ;; marked `id-exported?`. Mirrors typescript-passes.ss's predicate of
      ;; the same name. Non-exported ledger fields (e.g. tiny.compact's
      ;; `authority`, `state`) are dropped from the Rust public surface.
      (define (exported-public-binding? public-binding)
        (nanopass-case (Ltypescript Public-Ledger-Binding) public-binding
          [(,src ,ledger-field-name (,path-index* ...) ,type)
           (id-exported? ledger-field-name)]))

      ;; binding-field-name: extract the (snake-cased) Rust identifier for a
      ;; reader method from a public-binding.
      (define (binding-field-name public-binding)
        (nanopass-case (Ltypescript Public-Ledger-Binding) public-binding
          [(,src ,ledger-field-name (,path-index* ...) ,type)
           (camel->snake (id-sym ledger-field-name))]))

      ;; binding-path-indices: extract the list of path indices (integers)
      ;; that locate this field inside the on-chain state.
      (define (binding-path-indices public-binding)
        (nanopass-case (Ltypescript Public-Ledger-Binding) public-binding
          [(,src ,ledger-field-name (,path-index* ...) ,type) path-index*]))

      ;; binding-type: extract the binding's declared type (the ADT, e.g.
      ;; `(tadt Counter ...)` or `(tadt Cell ...)`).
      (define (binding-type public-binding)
        (nanopass-case (Ltypescript Public-Ledger-Binding) public-binding
          [(,src ,ledger-field-name (,path-index* ...) ,type) type]))

      ;; tadt-name=?: returns #t when `type` is a tadt with the given
      ;; adt-name (symbol). Used by emit-initial-state's R1/K1.1 dispatch
      ;; to pick the right per-ADT builder (new_map / new_merkle_tree /
      ;; new_historic_merkle_tree) instead of new_cell(Default::default()).
      (define (tadt-name=? type name)
        (nanopass-case (Ltypescript Type) type
          [(tadt ,src ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...))
           (eq? adt-name name)]
          [(talias ,src ,nominal? ,type-name ,type) (tadt-name=? type name)]
          [else #f]))

      ;; type-is-tvector?: returns #t when `type` is a tvector (after
      ;; de-aliasing). Used by emit-initial-state to route vector ledger
      ;; field seeds through new_cell_array (since `[T; N]: Into<AlignedValue>`
      ;; isn't impl'd upstream).
      (define (type-is-tvector? type)
        (nanopass-case (Ltypescript Type) type
          [(tvector ,src ,len ,type) #t]
          [(talias ,src ,nominal? ,type-name ,type) (type-is-tvector? type)]
          [else #f]))

      ;; tunsigned-bounded?: returns #t when `type` is a `tunsigned` whose
      ;; byte-length doesn't match the underlying Rust integer width's byte
      ;; count — i.e. a `Uint<L..U>` with a non-power-of-two byte-length.
      ;; Used by emit-initial-state to route those seeds through
      ;; `new_cell_bounded_uint(0u128, N)` so the on-state alignment
      ;; descriptor matches TS. Fixed-width `Uint<N>` (N ∈ {8,16,32,64,128})
      ;; stays on the `new_cell(0uN)` path.
      (define (tunsigned-bounded? type)
        (nanopass-case (Ltypescript Type) type
          [(tunsigned ,src ,nat) (not (uint-byte-length-matches-rust-width? nat))]
          [(talias ,src ,nominal? ,type-name ,type) (tunsigned-bounded? type)]
          [else #f]))

      ;; tunsigned-byte-length: byte-length of the `AlignmentAtom::Bytes`
      ;; descriptor for a `tunsigned` type. Mirrors TS's `byte-length` —
      ;; ceil(bit_length(max_value) / 8). Used by tunsigned-bounded? routing.
      (define (tunsigned-byte-length type)
        (nanopass-case (Ltypescript Type) type
          [(tunsigned ,src ,nat) (number->string (uint-byte-length nat))]
          [(talias ,src ,nominal? ,type-name ,type) (tunsigned-byte-length type)]
          [else "0"]))

      ;; tadt-merkle-height: given a MerkleTree / HistoricMerkleTree tadt,
      ;; extract the Nat height argument (the first adt-arg). The Public-
      ;; Ledger-ADT-Arg grammar permits either a nat or a type; for the
      ;; height position we expect a nat literal (see midnight-ledger.ss
      ;; declarations — `[Nat nat]` formal). Falls back to "32" if the
      ;; shape is unexpected so the emitter never crashes.
      (define (tadt-merkle-height type)
        (nanopass-case (Ltypescript Type) type
          [(tadt ,src ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...))
           (cond
             [(and (pair? adt-arg*) (number? (car adt-arg*)))
              (number->string (car adt-arg*))]
             [else "32"])]
          [(talias ,src ,nominal? ,type-name ,type) (tadt-merkle-height type)]
          [else "32"]))

      ;; tadt-read-op-type: given a binding's tadt, find the ADT operation
      ;; with op-class `read` and return its result type. Falls back to the
      ;; binding type itself if no read op is present (shouldn't happen for
      ;; ledger fields, but keep us robust).
      (define (tadt-read-op-type type)
        (nanopass-case (Ltypescript Type) type
          [(tadt ,src ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...))
           (let loop ([ops adt-op*])
             (cond
               [(null? ops) type]
               [else
                (let ([result
                       (nanopass-case (Ltypescript ADT-Op) (car ops)
                         [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
                          (if (eq? op-class 'read) type #f)])])
                  (or result (loop (cdr ops))))]))]
          [else type]))

      ;; decoder-for-type: pick the `midnight_compact_runtime::std_lib::decode_*`
      ;; helper that turns an AlignedValue back into the Rust type returned
      ;; by `type-rust`. Mirrors `uint-rust-width` for the integer cases.
      (define (decoder-for-type type)
        (nanopass-case (Ltypescript Type) type
          [(tunsigned ,src ,nat)
           (cond
             [(<= nat 255) "midnight_compact_runtime::std_lib::decode_u8"]
             [(<= nat 65535) "midnight_compact_runtime::std_lib::decode_u16"]
             [(<= nat 4294967295) "midnight_compact_runtime::std_lib::decode_u32"]
             [(<= nat 18446744073709551615) "midnight_compact_runtime::std_lib::decode_u64"]
             [else "midnight_compact_runtime::std_lib::decode_u128"])]
          [(tfield ,src ,ftype)
           ;; Native Field decodes via decode_fr. A JubjubScalar-typed
           ;; ledger read has no decoder yet (EmbeddedFr lacks
           ;; FromFieldRepr upstream and no fixture stores one); leave it
           ;; flagged (#f) so the caller reports the gap instead of
           ;; emitting wrong code.
           (if (field-type-native? ftype)
               "midnight_compact_runtime::std_lib::decode_fr"
               #f)]
          [(tboolean ,src) "midnight_compact_runtime::std_lib::decode_bool"]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
           ;; Bug-10 (2026-06-29): decode tenum-typed ledger reads as the
           ;; typed enum rather than as the raw u8 discriminant. Going
           ;; through `decode_via_field_repr::<EnumName>` (the user enum
           ;; derives FromFieldRepr via the H4 emission in
           ;; rust-passes-decls.ss) keeps both sides of an `==`/`!=`
           ;; comparison in the same Rust type. tiny's
           ;; `in_state(s: STATE)` body is `return state == s;` — the
           ;; LHS is this ledger read, the RHS is a STATE-typed formal,
           ;; and `decode_u8(_av)? == s` fails to compile (E0308). By
           ;; making the LHS a typed STATE, the walker's existing
           ;; tenum-name-of-type check on the `==` IR drives the RHS
           ;; enum-ref to render as `EnumName::variant`, eliminating
           ;; Bug-8's integer-coercion special case for tenum reads.
           ;; Boolean and Uint ledger reads still flow through their
           ;; integer decoders (decode_bool, decode_u8/u16/u32/u64/u128),
           ;; so Bug-8's short-circuit remains in force for those.
           (format "midnight_compact_runtime::std_lib::decode_via_field_repr::<~a>"
                   (symbol->string enum-name))]
          [(tbytes ,src ,len)
           (format "midnight_compact_runtime::std_lib::decode_bytes::<~a>" len)]
          [(tvector ,src ,len ,type)
           ;; Vector<N, T>: dispatch on element type. For Vector<N, Field>
           ;; and Vector<N, Uint<64>> we have dedicated decoders. Other
           ;; element types (Bytes<M>, user structs, nested vectors) need
           ;; their own helpers — leave them flagged so the gap is visible.
           (nanopass-case (Ltypescript Type) type
             [(tfield ,src ,ftype)
              (and (field-type-native? ftype)
                   (format "midnight_compact_runtime::std_lib::decode_vector_fr::<~a>" len))]
             [(tunsigned ,src ,nat)
              ;; Iter 7: Uint<64> element → decode_vector_u64<N>. Wider
              ;; widths (u128) and narrower (u8/u16/u32) would each need
              ;; their own per-element decoder; ship only the common case.
              (cond
                [(and (> nat 4294967295)
                      (<= nat 18446744073709551615))
                 (format "midnight_compact_runtime::std_lib::decode_vector_u64::<~a>" len)]
                [else #f])]
             [else #f])]
          [(talias ,src ,nominal? ,type-name ,type) (decoder-for-type type)]
          [(topaque ,src ,opaque-type)
           ;; JubjubPoint (EmbeddedGroupAffine) has no FromFieldRepr impl —
           ;; orphan rules forbid one downstream — so a JubjubPoint-typed
           ;; ledger read (did.compact 0.5.0's controllerPublicKey /
           ;; recoveryAuthorityPublicKey) cannot go through
           ;; decode_via_field_repr. Route it to the orphan-safe
           ;; decode_jubjub_point helper. Other opaque tags stay flagged.
           (if (equal? opaque-type "JubjubPoint")
               "midnight_compact_runtime::std_lib::decode_jubjub_point"
               #f)]
          [(tpoint ,src ,ctype)
           ;; 0.33: JubjubPoint is the builtin (tpoint (curve-jubjub));
           ;; same orphan-rule routing as the old topaque spelling.
           (nanopass-case (Ltypescript Curve-Type) ctype
             [(curve-jubjub) "midnight_compact_runtime::std_lib::decode_jubjub_point"]
             [else #f])]
          ;; A5: struct types (user-defined or stdlib like
          ;; `ContractAddress`) decode via the FromFieldRepr trait —
          ;; the H6/H7 emitter derives it for user structs, and
          ;; upstream stdlib structs in midnight-coin-structure /
          ;; midnight-base-crypto derive it natively. The Rust type
          ;; name comes through unqualified — it must already be in
          ;; scope at the call site (the codegen's `use
          ;; midnight_compact_runtime::*` import covers re-exported stdlib
          ;; types; user structs are emitted at module scope).
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (format "midnight_compact_runtime::std_lib::decode_via_field_repr::<~a>"
                   (struct-rust-name-of type struct-name))]
          [else #f]))

      ;; adt-is-collection?: ADTs whose `read` op-class is a per-element
      ;; presence/lookup check rather than a value extractor. For these,
      ;; the ledger view method (which currently decodes a single
      ;; AlignedValue → T) has incoherent semantics. Skip view emission
      ;; for collection-shaped ADTs; users access them through the typed
      ;; wrapper (E3 territory) or via direct StateValue inspection.
      (define (adt-is-collection? type)
        (nanopass-case (Ltypescript Type) type
          [(tadt ,src ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...))
           (memq adt-name '(Map Set MerkleTree HistoricMerkleTree List))]
          [else #f]))

      ;; default-value-rust: emit the Rust expression for a type's default
      ;; (zero) value. Used by emit-initial-state to seed each ledger field
      ;; before the constructor body runs. Mirrors `type-rust`'s structure.
      ;; - tunsigned → `0u<width>` matching uint-rust-width
      ;; - tfield → `Fr::default()` (Fr derives Default = zero)
      ;; - tboolean → `false`
      ;; - tbytes N → `[0u8; N]`
      ;; - tenum → `0u8` (the first variant's discriminant; works whether
      ;;   the enum is exported as a Rust type or not, since the on-chain
      ;;   FieldRepr is u8 regardless)
      ;; - else → `Default::default()` as a best-effort fallback.
      (define (default-value-rust type)
        (nanopass-case (Ltypescript Type) type
          [(tunsigned ,src ,nat) (format "0~a" (uint-rust-width nat))]
          [(tfield ,src ,ftype) (format "~a::default()" (field-type-rust ftype))]
          [(tpoint ,src ,ctype)
           ;; A23 equivalent for the 0.33 builtin point type: seed initial
           ;; cells with a concrete typed default so new_cell(...) infers.
           (nanopass-case (Ltypescript Curve-Type) ctype
             [(curve-jubjub) "midnight_compact_runtime::JubjubPoint::default()"]
             [else "Default::default()"])]
          [(tboolean ,src) "false"]
          [(tbytes ,src ,len) (format "[0u8; ~a]" len)]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...) "0u8"]
          [(tvector ,src ,len ,type)
           ;; Vector<N, T> defaults to an N-element array of T's default.
           ;; Requires T's default expression to be Copy (true for the
           ;; common primitives Fr/u*/bool/Bytes<M>).
           (format "[~a; ~a]" (default-value-rust type) len)]
          [(talias ,src ,nominal? ,type-name ,type) (default-value-rust type)]
          [(topaque ,src ,opaque-type)
           ;; Cat 4: give Opaque<"X"> a typed default so `new_cell(...)`
           ;; doesn't need explicit turbofish at the call site.
           ;; "string" goes through the OpaqueString newtype (orphan-rule
           ;; workaround for Aligned/FieldRepr on bare String); other
           ;; opaques stay as their direct mapping.
           (cond
             [(equal? opaque-type "string") "midnight_compact_runtime::std_lib::OpaqueString::default()"]
             [(equal? opaque-type "Uint8Array") "Vec::<u8>::new()"]
             ;; A23: JubjubPoint (EmbeddedGroupAffine) derives Default but the
             ;; generic `new_cell(Default::default())` can't infer the element
             ;; type, so seed the initial cell with a concrete typed default.
             [(equal? opaque-type "JubjubPoint") "midnight_compact_runtime::JubjubPoint::default()"]
             [else "Default::default()"])]
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           ;; Maybe<T> needs an explicit type parameter so `Default::default()`
           ;; can infer the payload. Other named structs derive Default
           ;; (H5 emits `#[derive(... Default)]`), so the bare struct name works.
           (cond
             [(eq? struct-name 'Maybe)
              (let loop ([names elt-name*] [types type*])
                (cond
                  [(null? names) "Maybe::<()>::default()"]
                  [(eq? (car names) 'value)
                   (format "Maybe::<~a>::default()" (type-rust (car types)))]
                  [else (loop (cdr names) (cdr types))]))]
             [else (format "~a::default()" (struct-rust-name-of type struct-name))])]
          ;; No catch-all. `Default::default()` here would be a guess about a
          ;; type we did not recognise: it either fails to compile (no Default
          ;; impl — E0277, in generated code) or compiles and seeds a cell with
          ;; a value that is not the Compact default. The second is the #45
          ;; failure mode, so refuse instead.
          ;;
          ;; `default-supported?` in the walker mirrors the arms above and is
          ;; meant to gate this, but it is consulted at one of the seven
          ;; `default-value-rust` call sites. Nothing reaches this arm today —
          ;; every route probed was closed by the decoder gate or the body
          ;; walker first — but that is defence by accident. This makes it
          ;; defence by construction.
          [else
           (rust-feature-error #f 'default-value-unsupported-type
             "no Rust default for this type; ~a"
             "seeding it with `Default::default()` would guess at the initial state")]))

      ;; emit-ledger-view: emits the module-level `ledger()` factory and the
      ;; `Ledger<'a, D>` view struct with one accessor method per *exported*
      ;; ledger field. Each method reads its field via a dup + N idx_at_index
      ;; ops + popeq Op program, then decodes the resulting AlignedValue
      ;; through the appropriate `decode_*` helper based on the binding's
      ;; ADT `read` op result type. The popeq uses ResultModeGather so the
      ;; read value is captured as a GatherEvent::Read(AlignedValue).
      (define (emit-ledger-view ledger-field*)
        (out "pub struct Ledger<'a, D: DB = DefaultDB> {\n")
        (out "    state: &'a ChargedState<D>,\n")
        (out "}\n\n")
        (out "pub fn ledger<D: DB>(state: &ChargedState<D>) -> Ledger<'_, D> {\n")
        (out "    Ledger { state }\n")
        (out "}\n\n")
        (out "impl<'a, D: DB> Ledger<'a, D> {\n")
        ;; Flatten ledger-declaration -> bindings, keep only exported ones.
        (let* ([all-bindings
                (apply append
                  (map (lambda (ldecl)
                         (nanopass-case (Ltypescript Program-Element) ldecl
                           [(public-ledger-declaration ,pl-array ,lconstructor)
                            (pl-array->public-bindings pl-array)]
                           [else '()]))
                       ledger-field*))]
               [exported-bindings
                (filter exported-public-binding? all-bindings)])
          (for-each
            (lambda (pb)
              ;; R4: skip collection-shaped ADTs (Map/Set/MerkleTree/HMT/List).
              ;; Their `read` op returns Boolean (presence check), not a value
              ;; extractor — emitting a `fn name(&self) -> Result<bool, ...>`
              ;; that ignores the key/element being checked produces a
              ;; nonsensical API. Direct StateValue inspection or the typed
              ;; wrapper (E3) is the right access path for these.
              (unless (adt-is-collection? (binding-type pb))
              (let* ([name (binding-field-name pb)]
                     [path* (binding-path-indices pb)]
                     [read-type (tadt-read-op-type (binding-type pb))]
                     [rust-ret (type-rust read-type)]
                     ;; A missing decoder used to fall back to `decode_u64`
                     ;; behind a TODO comment (MediaNoxLabs/compact#45). That
                     ;; emits a call whose return type cannot match the
                     ;; declared one — e.g. a `Vector<3, Bytes<32>>` field
                     ;; produced `decode_u64` in tail position of
                     ;; `Result<[[u8; 32]; 3], _>`, an E0308 — from a compile
                     ;; that exited 0. Reachable with the smallest possible
                     ;; contract: one ledger field, no circuits, no
                     ;; constructor. Refuse instead of guessing a decoder.
                     [decoder (or (decoder-for-type read-type)
                                  (rust-feature-error #f 'ledger-read-decoder-missing
                                    "no ledger-read decoder for `~a`; ~a"
                                    rust-ret
                                    "the accessor cannot be emitted without one"))])
                (out (format "    pub fn ~a(&self) -> Result<~a, CompactError> {\n" name rust-ret))
                ;; Bucket-1: see note in J2 emitter — fully-qualify the
                ;; upstream ContractAddress so user-defined shadow types
                ;; don't break QueryContext::new.
                (out "        let qctx = QueryContext::new(self.state.clone(), midnight_compact_runtime::ContractAddress::default());\n")
                (out "        let ops = OpProgramGather::<D>::new()\n")
                (out "            .dup(0)\n")
                (for-each
                  (lambda (idx)
                    (out (format "            .idx_at_index(~au8, false)\n" idx)))
                  path*)
                (out "            .popeq(true)\n")
                (out "            .build();\n")
                (out "        let results = query_for_read(&qctx, &ops, None, &initial_cost_model())\n")
                (out "            .map_err(|e| CompactError::AssertionFailed(format!(\"ledger query failed: {:?}\", e)))?;\n")
                (out "        let av = match results.events.last() {\n")
                (out "            Some(midnight_compact_runtime::onchain_vm::result_mode::GatherEvent::Read(av)) => av,\n")
                (out "            _ => return Err(CompactError::AssertionFailed(\"ledger: expected Read event\".into())),\n")
                (out "        };\n")
                (out (format "        ~a(av)\n" decoder))
                (out "    }\n"))))
            exported-bindings))
        (out "}\n\n"))

      ;; emit-pure-circuits: emits the `pure_circuits` module containing one
      ;; free function per pure circuit declaration. Contracts with no pure
      ;; circuits (e.g. counter.compact) get an empty module.
      ;;
      ;; When any pure circuit is present, inject `use super::*;` at the
      ;; top so the function bodies (and the `Result<T, CompactError>`
      ;; signatures, which reference `CompactError`) see crate-level
      ;; types — `CompactError`, `Maybe<T>`, user structs emitted by H5-H7,
      ;; the `compact_assert!` macro, etc. — without qualification.
      ;; `use super::*;` reaches the parent's `use midnight_compact_runtime::*;`
      ;; glob, so `CompactError` / `compact_assert!` resolve. Contracts
      ;; with no pure circuits (e.g. counter.compact) get an empty
      ;; module with no `use`.
      (define (emit-pure-circuits pure-circuit* native-id-ht
                                   witness-id-ht circuit-id-ht)
        (out "pub mod pure_circuits {\n")
        (let ([has-any? (not (null? pure-circuit*))])
          (when has-any?
            (out "    use super::*;\n\n")))
        (for-each (lambda (c) (emit-pure-circuit c native-id-ht
                                                       witness-id-ht circuit-id-ht))
                  pure-circuit*)
        (out "}\n"))

      ;; emit-cargo-toml: emits a Cargo.toml alongside lib.rs so users can
      ;; `cargo build` the emitted contract directly. The midnight-compact-runtime
      ;; dep is pinned to the same version the lib.rs embeds via
      ;; check_runtime_version!.
      (define (emit-cargo-toml)
        (let ([port (get-target-port 'contract-cargo.toml)])
          (display-string
            (format
              "[package]
name = \"compact-contract\"
version = \"0.1.0\"
edition = \"2021\"

[lib]
path = \"lib.rs\"

[dependencies]
midnight-compact-runtime = \"~a\"
"
              runtime-version-string)
            port)))
