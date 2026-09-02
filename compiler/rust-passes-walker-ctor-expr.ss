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

;;; Part of the former single-file `rust-passes-walker.ss` (3,792 lines),
;;; split by capability so each piece is reviewable on its own. The split
;;; is textual: every one of these files is `include`d into the same
;;; `(definitions ...)` block in rust-passes.ss, where internal defines are
;;; mutually recursive, so grouping carries no ordering constraint and no
;;; behavioural change. Byte parity over the fixture corpus is what proves
;;; that.
;;;
;;; This file: constructor-context expression rendering.

      (define (ctor-expr-rust expr local-binds
                              native-id-ht witness-id-ht circuit-id-ht)
        ;; Bug-1: thread local-binds through current-var-substitution so
        ;; sub-expressions that route through expr-rust (which doesn't take
        ;; local-binds) can still resolve formal→actual substitutions
        ;; introduced by inline-circuit-call. The dynamic binding mirrors
        ;; the explicit local-binds parameter — both must stay in sync.
        (parameterize ([current-var-substitution local-binds])
        (let ([e (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) e
            [(var-ref ,src ,var-name)
             (cond
               [(assq var-name local-binds) => cdr]
               [else (symbol->string (camel->snake (id-sym var-name)))])]
            [(enum-ref ,src ,type ,elt-name)
             (cond
               [(current-enum-ref-typed?)
                ;; M3.5: typed-enum context — render as `EnumName::r#variant`.
                ;; The `==` case below opts in to this rendering when the
                ;; other operand renders as a typed enum (e.g. a witness
                ;; call returning a tenum). Falls back to the integer
                ;; literal if the type isn't a recognised tenum.
                (or (enum-ref->typed-rust e)
                    (let ([n (enum-ref->u8 e)])
                      (if n (format "~au8" n)
                          "/* TODO M3-J2: unresolved enum-ref */ 0u8")))]
               [else
                (let ([n (enum-ref->u8 e)])
                  (if n (format "~au8" n)
                      "/* TODO M3-J2: unresolved enum-ref */ 0u8"))])]
            [(call ,src ,function-name ,expr* ...)
             (ctor-call-rust src function-name expr* local-binds
                             native-id-ht witness-id-ht circuit-id-ht)]
            [(default ,src ,type)
             ;; I3b/3: reuse the K1 zero-value helper.
             (default-value-rust type)]
            [(== ,src ,type ,expr1 ,expr2)
             ;; I3b/3: equality. Recurse via ctor-expr-rust so var-refs
             ;; still resolve through the current local-binds (the
             ;; inline-circuit-call formal substitution rides on this).
             ;;
             ;; M3.5: if either operand renders as a typed enum (currently
             ;; detected as: direct witness call whose return type is a
             ;; tenum), parameterize the recursion so any enum-ref on the
             ;; other side renders as `EnumName::variant` rather than the
             ;; integer discriminant. Election's
             ;; `private$state() == PrivateState.initial` hits this path:
             ;; the witness returns `PrivateState`, so `PrivateState.initial`
             ;; needs to be typed. The default integer rendering still
             ;; covers tiny.compact's `state == STATE.unset` (LHS is a
             ;; u8-decoded ledger read) and election's
             ;; `state.read() == PublicState.commit` (same: ledger decoder
             ;; produces u8).
             ;;
             ;; Bug-4 (2026-06-24): the `==` IR node carries the resolved
             ;; comparison `type` directly. When that type is itself a
             ;; tenum (did.compact's `disclosed_mutation == MapMutation.Insert`
             ;; with disclosed_mutation a `const = disclose(mutation)` whose
             ;; declared type is MapMutation), neither operand-side
             ;; heuristic fires (the var-ref isn't a formal arg and
             ;; `disclose` isn't a witness/circuit), but the IR's type
             ;; field is conclusive. Prefer it.
             ;; Bug-8 (2026-06-26): if either operand is a ledger-read,
             ;; the Rust decoder produces a raw u8 (see rust-passes-emit
             ;; `state.read()` -> `decode_u8`) regardless of the
             ;; declared Compact tenum type. Election's
             ;; `state.read() == PublicState.commit` regressed when
             ;; Bug-4 added the `tenum-name-of-type type` check: the
             ;; IR's `==` type is `PublicState`, so typed rendering
             ;; fired on both sides → `u8 == PublicState::commit`,
             ;; which doesn't compile. Short-circuit to integer
             ;; rendering so the enum-ref drops to its u8 discriminant.
             ;; The walker comment above (~line 188) already documented
             ;; this intent; Bug-4 contradicted it.
             (let ([typed?
                    (and (not (is-ledger-read-expr? expr1))
                         (not (is-ledger-read-expr? expr2))
                         (or (tenum-name-of-type type)
                             (operand-typed-enum? expr1 witness-id-ht)
                             (operand-typed-enum? expr2 witness-id-ht)))])
               (parameterize ([current-enum-ref-typed? typed?])
                 (format "(~a == ~a)"
                         (coerce-cmp-operand-rust expr1 type local-binds
                                                   native-id-ht witness-id-ht circuit-id-ht)
                         (coerce-cmp-operand-rust expr2 type local-binds
                                                   native-id-ht witness-id-ht circuit-id-ht))))]
            [(!= ,src ,type ,expr1 ,expr2)
             ;; A9: inequality — same typed-enum threading as `==`. Bug-4:
             ;; also prefer the IR type when it's a tenum. Bug-8: same
             ;; ledger-read short-circuit as `==` (see the long note above).
             (let ([typed?
                    (and (not (is-ledger-read-expr? expr1))
                         (not (is-ledger-read-expr? expr2))
                         (or (tenum-name-of-type type)
                             (operand-typed-enum? expr1 witness-id-ht)
                             (operand-typed-enum? expr2 witness-id-ht)))])
               (parameterize ([current-enum-ref-typed? typed?])
                 (format "(~a != ~a)"
                         (coerce-cmp-operand-rust expr1 type local-binds
                                                   native-id-ht witness-id-ht circuit-id-ht)
                         (coerce-cmp-operand-rust expr2 type local-binds
                                                   native-id-ht witness-id-ht circuit-id-ht))))]
            [(not ,src ,expr)
             ;; F1.2: Boolean negation. Recurse through ctor-expr-rust to
             ;; keep local-binds threading. Parenthesise the operand so
             ;; the surrounding context can compose safely.
             (format "(!(~a))"
                     (ctor-expr-rust expr local-binds
                                     native-id-ht witness-id-ht circuit-id-ht))]
            [(and ,src ,expr1 ,expr2)
             ;; F1.2: short-circuit Boolean AND.
             (format "(~a && ~a)"
                     (ctor-expr-rust expr1 local-binds
                                     native-id-ht witness-id-ht circuit-id-ht)
                     (ctor-expr-rust expr2 local-binds
                                     native-id-ht witness-id-ht circuit-id-ht))]
            [(or ,src ,expr1 ,expr2)
             ;; F1.2: short-circuit Boolean OR.
             (format "(~a || ~a)"
                     (ctor-expr-rust expr1 local-binds
                                     native-id-ht witness-id-ht circuit-id-ht)
                     (ctor-expr-rust expr2 local-binds
                                     native-id-ht witness-id-ht circuit-id-ht))]
            [(elt-ref ,src ,expr ,elt-name ,nat)
             ;; F1.2: struct field access (`struct.field`). The field
             ;; name comes from the source language; rust-variant-name
             ;; handles reserved-word escapes.
             ;;
             ;; F2.2: special-case `(elt-ref (public-ledger ... read) f 0)`
             ;; when the read returns a tstruct — the whole-struct
             ;; decoder doesn't exist (e.g. Maybe<Opaque<string>>), but
             ;; the first field's decoder applies directly to the
             ;; gathered AlignedValue (its layout starts with the
             ;; leading field's atoms). We render a gather block + the
             ;; field-0 decoder, skipping the intermediate
             ;; `<struct>.field` projection.
             (cond
               [(and (fx= nat 0)
                     (elt-ref-of-struct-read? expr))
                (emit-struct-field-zero-read
                  (struct-read-path-elts expr)
                  (struct-read-first-field-decoder expr))]
               [else
                (format "~a.~a"
                        (ctor-expr-rust expr local-binds
                                        native-id-ht witness-id-ht circuit-id-ht)
                        (symbol->string elt-name))])]
            [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
             ;; I3b/3: ledger read in expression position. Goes through
             ;; emit-ledger-read-expr which uses current-qctx-ref to pick
             ;; the right QueryContext source. F1.2/2: passes expr* +
             ;; native-id-ht so ADT read-with-arg (Set.member, HMT.
             ;; checkRoot, Map.lookup) routes through the vm-code
             ;; expansion path.
             (emit-ledger-read-expr path-elt* adt-op expr* native-id-ht)]
            [(new ,src ,type ,expr* ...)
             ;; F2.2: struct-literal construction
             ;; (`Maybe<...>{ is_some: true, value: t }` or
             ;; `UserStruct{ f1: v1, ... }`). The IR carries the field
             ;; initialisers in source order; field names come off the
             ;; type. Maybe<T> renders as the L1 alias `Maybe`; other
             ;; tstructs render as their bare name (H5-H7 emit them as
             ;; concrete Rust structs in the contract module).
             (render-struct-literal
               src type expr* local-binds
               native-id-ht witness-id-ht circuit-id-ht)]
            [(map ,src ,len ,fun ,map-arg ,map-arg* ...)
             ;; Iter 7: `map(fn, iterable)` over a static-length literal
             ;; iterable. expr-supported? has already validated the
             ;; shape (single map-arg, bare-lambda over a single param,
             ;; iterable peels to a static tuple/vector literal, body
             ;; is identity). We render by per-element substitution:
             ;; for each literal in the iterable, substitute the param
             ;; with the literal in the body and render via expr-rust.
             ;; The result is a Rust array literal `[<v0>, <v1>, ...]`
             ;; with type `[T; N]`, which `new_cell_array` consumes
             ;; (the constructor-body walker routes Vector<N,T> writes
             ;; through `new_cell_array` via `current-ledger-field-types`).
             (render-map-mvp src fun map-arg native-id-ht)]
            [else
             ;; quote/tuple/etc. fall through to the existing expr-rust.
             (expr-rust e native-id-ht)]))))

      ;; render-map-mvp: render a `(map src len fun map-arg)` IR node
      ;; as a Rust array literal `[v0, v1, ..., vN-1]`. Assumes
      ;; `map-expr-mvp-supported?` accepted the shape — i.e. fun is a
      ;; single-param bare lambda whose body is identity (a var-ref
      ;; to the param after expr-strip-cast), and map-arg's expr peels
      ;; to a static-tuple/vector literal whose elements are
      ;; integer-literal Expressions (per iterable-expr->literals).
      ;;
      ;; For identity-body, each iteration's rendered value is just
      ;; expr-rust on the i-th literal (so the resulting array is
      ;; the iterable's bare elements). Non-identity bodies are
      ;; rejected by map-expr-mvp-supported?; future iterations can
      ;; extend this helper to substitute the param into a richer
      ;; body via expr-subst-var-ref + expr-rust.
      (define (render-map-mvp src fun map-arg native-id-ht)
        (let* ([iter-expr (map-arg->expr map-arg)]
               [literals (iterable-expr->literals iter-expr)]
               [param-name (lambda-param-name fun)]
               [body (lambda-body-expr fun)]
               [ret-type (lambda-return-type fun)]
               ;; Type suffix for the leading literal so Rust can infer
               ;; the array's element type without an explicit `[T; N]`
               ;; ascription. Identity lambdas with a tunsigned return
               ;; type get `u<width>` (u8/u16/u32/u64/u128); other
               ;; element types fall back to no suffix (Rust will
               ;; default integer literals to i32 — that's OK for
               ;; `[i32; N]` consumers but `new_cell_array` only impls
               ;; `Into<AlignedValue>` for the specific widths upstream
               ;; supports, so a missing suffix here surfaces as a
               ;; type-check error in the generated code rather than
               ;; silently producing wrong Rust).
               ;;
               ;; Iter 7 follow-up: non-identity bodies already end in a
               ;; `... as u<width>` from the body's `downcast-unsigned`
               ;; rendering, so they don't need (and would be broken by)
               ;; the leading-element suffix. We detect this by
               ;; inspecting the body shape: identity bodies get the
               ;; suffix; bodies with arithmetic / downcast typing
               ;; suppress it.
               [first-suffix (if (lambda-body-identity? body param-name)
                                 (or (tunsigned-rust-suffix ret-type) "")
                                 "")])
          (cond
            [(or (not literals) (not param-name) (not body))
             (rust-feature-error src 'map-mvp-shape
               "map: shape changed between expr-supported? and ctor-expr-rust")]
            [else
             ;; Per-iteration: substitute param-name with the i-th
             ;; literal in the body (works trivially for the identity
             ;; body — the substituted Expression is just the literal
             ;; itself, possibly under safe-cast layers). Render via
             ;; expr-rust which handles `(quote ,src ,int)` shape. The
             ;; FIRST element gets the `u<width>` suffix so Rust infers
             ;; the array's element type; subsequent elements stay
             ;; bare (Rust infers them from the array's element type).
             (let ([rendered
                    (map (lambda (lit)
                           (let ([substituted
                                  (expr-subst-var-ref body param-name lit)])
                             (expr-rust substituted native-id-ht)))
                         literals)])
               (string-append
                 "["
                 (let join ([xs rendered] [first? #t] [acc ""])
                   (cond
                     [(null? xs) acc]
                     [else
                      (let ([part (if first?
                                      (string-append (car xs) first-suffix)
                                      (car xs))])
                        (cond
                          [(null? (cdr xs)) (string-append acc part)]
                          [else (join (cdr xs) #f
                                      (string-append acc part ", "))]))]))
                 "]"))])))

      ;; lambda-return-type: from a Function IR node, return the
      ;; declared return Type. Returns #f when the shape isn't a
      ;; `(circuit src args type stmt)`.
      (define (lambda-return-type fun)
        (nanopass-case (Ltypescript Function) fun
          [(circuit ,src (,arg* ...) ,type ,stmt) type]
          [else #f]))

      ;; tunsigned-rust-suffix: for a `(tunsigned ,nat)` Type, return
      ;; the matching Rust integer suffix string ("u8" / "u16" / ...
      ;; "u128"). Returns #f for any other type or for tunsigned widths
      ;; outside the 8/16/32/64/128 ladder (Uint<L..U> bounded ranges
      ;; need their own width handling — Iter 8's bounded-uint fixture
      ;; uses `new_cell_bounded_uint`, not `new_cell_array`, so we
      ;; don't intercept here). Mirrors decoder-for-type's tunsigned
      ;; ladder for consistency.
      (define (tunsigned-rust-suffix type)
        (nanopass-case (Ltypescript Type) type
          [(tunsigned ,src ,nat)
           (tunsigned-rust-suffix-for-bound nat)]
          [(talias ,src ,nominal? ,type-name ,type) (tunsigned-rust-suffix type)]
          [else #f]))

      ;; tunsigned-rust-suffix-for-bound: given a raw unsigned upper bound
      ;; (e.g. 100 for `Uint<0..100>`, or 18446744073709551615 = u64::MAX
      ;; for `Uint<64>`), return the smallest Rust integer suffix string
      ;; whose width can hold `nat` ("u8" / "u16" / "u32" / "u64" /
      ;; "u128"). Returns #f only when `nat` exceeds u128::MAX or is
      ;; negative.
      ;;
      ;; Bug-11 (2026-06-29): generalised from the
      ;; {255,65535,4294967295,u64::MAX,u128::MAX} power-of-2-minus-1
      ;; ladder to accept any nat ≤ u128::MAX. The Compact language
      ;; spec sanctions arbitrary `Uint<L..U>` upper bounds, and the
      ;; TS backend handles them via `CompactTypeUnsignedInteger`'s
      ;; arbitrary `length` parameter. Pre-Bug-11 we rejected
      ;; non-power-of-2 nats here, which propagated up as a walker
      ;; "no walker shape matched" error for any non-literal write to
      ;; a bounded-range field (e.g. `n.write(s.size() as Uint<0..100>)`).
      ;; The narrower-than-Rust-width byte-length is handled by routing
      ;; the cell builder through `new_cell_bounded_uint` (see
      ;; emit-body-mutations / emit-body-writes).
      (define (tunsigned-rust-suffix-for-bound nat)
        (cond
          [(or (not (integer? nat)) (not (exact? nat)) (negative? nat)) #f]
          [(<= nat 255) "u8"]
          [(<= nat 65535) "u16"]
          [(<= nat 4294967295) "u32"]
          [(<= nat 18446744073709551615) "u64"]
          [(<= nat 340282366920938463463374607431768211455) "u128"]
          [else #f]))

      ;; arg-rust-clone-if-var: render a call argument expression and
      ;; suffix `.clone()` if the rendered form is a bare var-ref. Used
      ;; by E4.4's bare-call emitter so passing the same Compact-level
      ;; var as an argument twice (e.g. `private$add_coin(coin)` followed
      ;; by `pure_circuits::commitment_from_coin_info(coin, pk)`) doesn't
      ;; trip Rust's move semantics. Defensive: we don't have liveness
      ;; analysis, so we clone every var-ref arg. User structs derive
      ;; Clone (H5), so the clone is a no-op semantically and cheap for
      ;; the small struct shapes Compact emits.
      (define (arg-rust-clone-if-var e local-binds
                                     native-id-ht witness-id-ht circuit-id-ht)
        (let ([rendered (ctor-expr-rust e local-binds
                                        native-id-ht witness-id-ht circuit-id-ht)])
          (let ([stripped (expr-strip-cast e)])
            (nanopass-case (Ltypescript Expression) stripped
              [(var-ref ,src ,var-name)
               (if (var-ref-known-copy? var-name)
                   rendered
                   (string-append rendered ".clone()"))]
              ;; elt-ref of a (chain of) var-ref(s): `path.value`,
              ;; `dest_public_key.zk` etc. Borrow-of-moved-value errors
              ;; surface when the same field is passed to one call then
              ;; re-read after — defensive clone keeps the owner intact.
              ;; We clone whenever the leaf field's type is unknown or
              ;; non-Copy; for known-Copy field types we skip the clone.
              [(elt-ref ,src ,expr ,elt-name ,nat)
               (cond
                 [(not (elt-ref-rooted-in-var? stripped)) rendered]
                 [(elt-ref-known-copy? stripped) rendered]
                 [else (string-append rendered ".clone()")])]
              [else rendered]))))

      ;; var-ref-known-copy?: returns #t when the local has a declared
      ;; type recorded in `current-formal-arg-types` that lowers to a
      ;; Rust `Copy` type. Locals without a recorded type default to #f
      ;; (caller will clone — safe and a no-op for Copy types in
      ;; practice, but explicit knowledge keeps emitted code clean).
      (define (var-ref-known-copy? var-name)
        (let ([ht (current-formal-arg-types)])
          (and ht
               (let ([t (eq-hashtable-ref ht (id-sym var-name) #f)])
                 (and t (type-rust-copy? t))))))

      ;; elt-ref-rooted-in-var?: walk an elt-ref chain to its base; return
      ;; #t when the chain's leftmost expression is a var-ref (i.e. a
      ;; field projection on a local). Used by arg-rust-clone-if-var so
      ;; passing `path.value` as a call arg suffixes `.clone()`.
      (define (elt-ref-rooted-in-var? e)
        (nanopass-case (Ltypescript Expression) e
          [(var-ref ,src ,var-name) #t]
          [(elt-ref ,src ,expr ,elt-name ,nat)
           (elt-ref-rooted-in-var? (expr-strip-cast expr))]
          [else #f]))

      ;; elt-ref-known-copy?: walk an elt-ref to the base var; if the
      ;; base var has a recorded struct type, project to the named field
      ;; and check whether THAT field's type is Copy. Returns #f when any
      ;; step is unknown (caller defaults to cloning).
      (define (elt-ref-known-copy? e)
        (let walk ([n e])
          (nanopass-case (Ltypescript Expression) n
            [(var-ref ,src ,var-name)
             (let ([ht (current-formal-arg-types)])
               (and ht
                    (let ([t (eq-hashtable-ref ht (id-sym var-name) #f)])
                      (and t (type-rust-copy? t)))))]
            [(elt-ref ,src ,expr ,elt-name ,nat)
             ;; If inner is a known-Copy struct it doesn't matter (Copy
             ;; struct -> Copy fields). Otherwise we'd need to descend
             ;; into struct definitions to type the projected field; for
             ;; now, return #f (clone). This keeps the predicate
             ;; conservative — extra clones on field projections are
             ;; always safe.
             (walk (expr-strip-cast expr))]
            [else #f])))

      ;; stdlib-circuit-rust-path: if `function-name` is a Compact stdlib
      ;; circuit registered in stdlib-circuit-mappings, return its
      ;; runtime-side Rust callee path (with `::<T>` ascription where the
      ;; mapping needs it). Returns #f for non-stdlib callees.
      ;;
      ;; `cdefn` is the circuit pelt looked up via circuit-id-ht (or #f),
      ;; passed through to the mapping's rust-path-fn so it can inspect
      ;; the return type (e.g. some/none extract T from `Maybe<T>`).
      (define (stdlib-circuit-rust-path function-name cdefn)
        (let ([entry (lookup-stdlib-circuit (id-sym function-name))])
          (and entry ((car entry) cdefn))))

      ;; circuit-return-type: pull the declared return type out of a
      ;; Circuit-Definition Program-Element. Returns #f if `cdefn` is not
      ;; a circuit (defensive).
      (define (circuit-return-type cdefn)
        (nanopass-case (Ltypescript Program-Element) cdefn
          [(circuit ,src ,function-name (,arg* ...) ,type ,stmt) type]
          [else #f]))

      ;; ctor-call-rust: render a `(call f args)` in the constructor body.
      ;; The witness case is special — witnesses don't appear as
      ;; sub-expressions in the constructor body, they always sit on the RHS
      ;; of a `const` binding, so the witness branch here just renders the
      ;; method call with a witness-context placeholder name we'll have
      ;; emitted on the line above. Pure circuit calls resolve to
      ;; `pure_circuits::<name>(...)`. Native and other circuit calls fall
      ;; back to call-rust.
      (define (ctor-call-rust src function-name expr* local-binds
                              native-id-ht witness-id-ht circuit-id-ht)
        (let* ([w (eq-hashtable-ref witness-id-ht function-name #f)]
               [c (eq-hashtable-ref circuit-id-ht function-name #f)]
               [stdlib (stdlib-circuit-rust-path function-name c)])
          (cond
            [w
             ;; Witness calls cannot be inlined as sub-expressions because
             ;; they return (PS, T). The body walker hoists witness calls
             ;; nested in assert-conditions to top-level `let`-bindings;
             ;; consult current-witness-call-binds (an alist keyed by
             ;; function-name + arg expr*) before falling back to a TODO.
             ;; We compare arg lists by eq?-on-each since assert-cond
             ;; rendering walks the same IR nodes the hoister scanned.
             (cond
               [(witness-call-bound function-name expr*
                                    (current-witness-call-binds))
                => (lambda (rust-name) rust-name)]
               [else
                (rust-feature-error src 'witness-inline
                  "witness call ~a appears as a sub-expression; only top-level binding shape is supported"
                  (id-sym function-name))])]
            [stdlib
             ;; I3b/4: stdlib circuits (`some`, `none`) live in
             ;; midnight_compact_runtime::std_lib. Render with the runtime path.
             (let ([args
                    (map (lambda (e)
                           (arg-rust-clone-if-var e local-binds
                                                  native-id-ht witness-id-ht circuit-id-ht))
                         expr*)])
               (format "~a(~a)"
                       stdlib
                       (let join ([xs args] [acc ""])
                         (cond
                           [(null? xs) acc]
                           [(null? (cdr xs)) (string-append acc (car xs))]
                           [else (join (cdr xs)
                                       (string-append acc (car xs) ", "))]))))]
            [(and c (id-pure? function-name))
             (let ([rust-name (id->rust-name function-name)]
                   [args
                    (map (lambda (e)
                           (arg-rust-clone-if-var e local-binds
                                                  native-id-ht witness-id-ht circuit-id-ht))
                         expr*)])
               ;; Append `?` to unwrap the `Result<T, CompactError>` a
               ;; pure circuit returns. Every position reaching here
               ;; (cond-rust/assert-cond-rust conditions, if-branch
               ;; expressions, inlined bodies) lives inside a function
               ;; returning `Result<_, CompactError>`, so `?` is valid.
               ;; Matches call-rust's pure-circuit arm in rust-passes-emit.
               (format "pure_circuits::~a(~a)?"
                       rust-name
                       (let join ([xs args] [acc ""])
                         (cond
                           [(null? xs) acc]
                           [(null? (cdr xs)) (string-append acc (car xs))]
                           [else (join (cdr xs)
                                       (string-append acc (car xs) ", "))]))))]
            [else
             ;; A15: if this call was hoisted before the surrounding
             ;; assert (the streaming walker's assert clause and the A12
             ;; if-then-else emit clause scan for non-pure user-circuit
             ;; calls in assert conds and lift them to `let _cr_hN =
             ;; self.X(ctx, ...)?` bindings), render as a reference to
             ;; the hoisted result. Otherwise fall to existing call-rust
             ;; (which errors with "non-native-call" on a non-pure user
             ;; circuit — the same diagnostic compactc used pre-A15).
             (cond
               [(impure-call-bound function-name expr*
                                   (current-impure-call-binds))
                => (lambda (rust-name)
                     (format "~a.result.clone()" rust-name))]
               [else
                (call-rust src function-name expr* native-id-ht)])])))
