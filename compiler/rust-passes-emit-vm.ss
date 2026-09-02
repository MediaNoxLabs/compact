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
;;; This file: vm-value lowering and op-program builder calls.

      ;; path-elt->vm-value: turn a Path-Element into the VM value that
      ;; `expand-vm-code` expects to see in the `f` argument. For path
      ;; indices (constant nats locating a ledger field) we emit a
      ;; `(VMalign path-index 1)` exactly as typescript-passes.ss does
      ;; (see line 695). Typed path elements (`(src type expr)`) — used
      ;; when the path includes a runtime expression, e.g. a Map key —
      ;; are not part of the I3a wedge; we return #f so the caller falls
      ;; back to `unimplemented!()` for those.
      (define (path-elt->vm-value path-elt)
        (nanopass-case (Ltypescript Path-Element) path-elt
          [,path-index (VMalign path-index 1)]
          [else #f]))

      ;; vm-rust-expr is a thin carrier record used to ferry a pre-rendered
      ;; Rust expression string through `expand-vm-code`. It lets the I3a
      ;; entry collapse non-literal circuit arguments (e.g. a Bytes32 var-ref
      ;; like `pk` or a pure-circuit result like `cm`) into a single Scheme
      ;; value that survives macro expansion intact, then surfaces back out
      ;; at `vminstr->builder-call` time so the push for that arg lowers to
      ;; the right Rust expression. Counter's literal-int path is preserved
      ;; unchanged (integers are still plain Scheme numbers).
      (define-record-type vm-rust-expr
        (nongenerative)
        (fields text))

      ;; expr->vm-value: turn a circuit argument Expression into a value
      ;; that the VM code can consume. Counter's `round.increment(1)`
      ;; passes the constant `1`; the vm-code wraps that in
      ;; `(rt-value->int amount)`, producing `(VMvalue->int <int>)` after
      ;; expansion, which we unwrap in vminstr->builder-call. For E4 the
      ;; insert ops pass a non-literal (e.g. `pk` / `cm`), so we also
      ;; accept any expression `expr-rust` can render and lift it into a
      ;; `vm-rust-expr` carrier — vminstr->builder-call recognises the
      ;; carrier when surfacing the push value. Returns #f only when the
      ;; expression itself can't be rendered.
      ;;
      ;; `native-id-ht` lets us route var-refs to native bindings (e.g.
      ;; arg names that came in via emit-circuit-args) when needed; the
      ;; counter literal-only path doesn't need it (and historically
      ;; didn't take it), so it defaults to #f and we only invoke
      ;; expr-rust when the expression isn't a plain literal.
      (define (expr->vm-value expr . opt-native-ht)
        (nanopass-case (Ltypescript Expression) expr
          [(quote ,src ,datum)
           (if (and (integer? datum) (exact? datum)) datum #f)]
          [else
           (let ([native-ht (and (pair? opt-native-ht) (car opt-native-ht))])
             (and native-ht
                  (let ([rendered (guard (c [#t #f]) (expr-rust expr native-ht))])
                    (and rendered
                         (make-vm-rust-expr
                           (expr-rust-arg-cloned expr rendered))))))]))

      ;; expr-rust-arg-cloned: given the original expression and its
      ;; expr-rust-rendered text, suffix `.clone()` when the expression
      ;; is a var-ref / elt-ref to a non-Copy local. Mirrors
      ;; arg-rust-clone-if-var's predicate (without re-rendering, since
      ;; expr-rust has already produced the text). Used by the
      ;; assert-cond / read-with-arg path where `expr-rust` (not
      ;; `ctor-expr-rust`) produces the inner text.
      (define (expr-rust-arg-cloned expr rendered)
        (let ([stripped (expr-strip-cast expr)])
          (nanopass-case (Ltypescript Expression) stripped
            [(var-ref ,src ,var-name)
             (if (var-ref-known-copy? var-name)
                 rendered
                 (string-append rendered ".clone()"))]
            [(elt-ref ,src ,expr ,elt-name ,nat)
             (cond
               [(not (elt-ref-rooted-in-var? stripped)) rendered]
               [(elt-ref-known-copy? stripped) rendered]
               [else (string-append rendered ".clone()")])]
            [else rendered])))

      ;; vm-immediate->int: given the value the vm-code computed for an
      ;; `[immediate ...]` argument, unwrap a `(VMvalue->int n)` (the
      ;; standard wrap produced by `(rt-value->int amount)`) into the
      ;; underlying integer. Returns #f if the value isn't a plain
      ;; literal-flavoured immediate.
      (define (vm-immediate->int v)
        (cond
          [(and (integer? v) (exact? v)) v]
          [(VMop? v)
           (VMop-case v
             [(VMvalue->int x)
              (if (and (integer? x) (exact? x)) x #f)]
             [else #f])]
          [else #f]))

      ;; vm-path->indices: given the value bound to `path` (typically the
      ;; whole `f` list in the vm-code), return a list of integer indices
      ;; if every element is a `(VMalign nat 1)`. Returns #f otherwise so
      ;; richer paths fall back to `unimplemented!()`.
      (define (vm-path->indices v)
        (cond
          [(not (list? v)) #f]
          [else
           (let loop ([xs v] [acc '()])
             (cond
               [(null? xs) (reverse acc)]
               [(VMop? (car xs))
                (VMop-case (car xs)
                  [(VMalign value bytes)
                   (if (and (= bytes 1) (integer? value) (exact? value))
                       (loop (cdr xs) (cons value acc))
                       #f)]
                  [else #f])]
               [else #f]))]))

      ;; vm-cell-elem->rust: render the inner expression of a
      ;; (VMstate-value-cell <elem>) form as the Rust expression that
      ;; should be wrapped in `new_cell(...)`. Recognises:
      ;;   - a plain integer literal              → "<n>u8"  (counter-style)
      ;;   - a VMalign with bytes=1               → "<n>u8"
      ;;   - a vm-rust-expr carrier               → the carrier's text
      ;;   - a VMleaf-hash wrapping any of the above
      ;;       → "leaf_hash(&ValueReprAlignedValue(AlignedValue::from(<inner>)))"
      ;;     (matches midnight-onchain-runtime's program_fragments shape)
      ;; Returns #f for anything we don't yet know how to render so the
      ;; caller can fall back to `unimplemented!()`.
      (define (vm-cell-elem->rust elem)
        (cond
          [(and (integer? elem) (exact? elem))
           (format "~au8" elem)]
          [(vm-rust-expr? elem)
           (vm-rust-expr-text elem)]
          [(VMop? elem)
           (VMop-case elem
             [(VMalign value bytes)
              ;; A20: extend beyond bytes=1 (Counter / Cell.read literals)
              ;; to support the wider literals that read-no-arg vm-code
              ;; pushes — e.g. `Set.isEmpty` emits `(push align-0-8)` for a
              ;; u64 zero compared against `(size)`. The 1/2/4/8 widths
              ;; match the on-state alignment widths of Compact's
              ;; Uint8 / Uint16 / Uint32 / Uint64 cells and map 1:1 onto
              ;; Rust's primitive `From<…> for AlignedValue` impls.
              (cond
                [(not (and (integer? value) (exact? value))) #f]
                [(= bytes 1) (format "~au8" value)]
                [(= bytes 2) (format "~au16" value)]
                [(= bytes 4) (format "~au32" value)]
                [(= bytes 8) (format "~au64" value)]
                [else #f])]
             [(VMleaf-hash x)
              (let ([inner (vm-cell-elem->rust x)])
                (and inner
                     (format
                       "leaf_hash(&ValueReprAlignedValue(AlignedValue::from(~a)))"
                       inner)))]
             ;; A21: `(rt-null T)` lowers to `VMnull T` carrying the type
             ;; whose default value we need to materialise. Used by HMT
             ;; (and MerkleTree) `insertIndexDefault` to push a hash of the
             ;; default leaf — `(rt-leaf-hash (rt-null value_type))` — and
             ;; later by any vm-code that needs a typed zero. We route the
             ;; type through `default-value-rust` (the same renderer
             ;; `emit-initial-state` uses for the ledger field zero) so the
             ;; Rust literal matches the TS reference's
             ;; `default(value_type)` expression bit-for-bit.
             [(VMnull type)
              (default-value-rust type)]
             [else #f])]
          [else #f]))

      ;; ltypescript-type-is-tadt?: returns #t when an Ltypescript Type is a
      ;; tadt (public-adt) form, peeling talias layers. Used by
      ;; vm-value->rust-state-value below to decide whether a
      ;; `VMstate-value-ADT val type` form wraps its `val` in `new_cell(...)`
      ;; (the leaf path — value_type is a non-ADT like Field/Uint/Bytes) or
      ;; surfaces the ADT directly (the recursive path — value_type is a
      ;; nested Map/Set/MerkleTree). Mirrors typescript-passes.ss's
      ;; `public-adt?` branch for `VMstate-value-ADT`.
      (define (ltypescript-type-is-tadt? type)
        (nanopass-case (Ltypescript Type) type
          [(tadt ,src ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...)) #t]
          [(talias ,src ,nominal? ,type-name ,type) (ltypescript-type-is-tadt? type)]
          [else #f]))

      ;; vm-value->rust-state-value: render a state-value form (the kind
      ;; that appears as the `value` arg of a `push` vm-instruction) as
      ;; a Rust expression of type `StateValue<DefaultDB>`. Returns #f if
      ;; the form isn't one we yet know how to translate.
      ;;
      ;; G: `VMstate-value-ADT val type` — Map.insert's value-side push uses
      ;; this form because `state-value 'ADT value value_type` carries the
      ;; declared value_type. For non-public-adt value types (Field, Uint*,
      ;; Bytes<N>, …) the runtime semantics are identical to a plain
      ;; `VMstate-value-cell val` (typescript-passes wraps with
      ;; `StateValue.newCell`); we mirror that here so Map<K, leaf> insert
      ;; lowers to the same `.push(true, new_cell(<v>))` shape Set.insert
      ;; uses with its `(state-value 'null)`. tadt-typed values aren't yet
      ;; needed (Map<K, Map<…>> etc.) — return #f for those so the caller
      ;; falls back to `unimplemented!()`.
      (define (vm-value->rust-state-value val)
        (cond
          [(VMop? val)
           (VMop-case val
             [(VMstate-value-null) "StateValue::Null"]
             [(VMstate-value-cell inner)
              (let ([rust-inner (vm-cell-elem->rust inner)])
                (and rust-inner (format "new_cell(~a)" rust-inner)))]
             [(VMstate-value-ADT inner adt-type)
              (cond
                [(ltypescript-type-is-tadt? adt-type) #f]
                [else
                 (let ([rust-inner (vm-cell-elem->rust inner)])
                   (and rust-inner (format "new_cell(~a)" rust-inner)))])]
             [else #f])]
          [else #f]))

      ;; vminstr->builder-call: render a single vminstr as one line of the
      ;; OpProgramVerify builder chain (already indented for inclusion
      ;; inside the `let ops = ...` block). Recognises the ops needed by
      ;; counter (`idx`, `addi`, `ins`) plus the vm-ops emitted by Set /
      ;; MerkleTree / HistoricMerkleTree `insert` vm-code (`push`, `dup`,
      ;; `root`). Anything else returns #f so the caller can bail out to
      ;; the `unimplemented!()` fallback rather than emit syntactically-
      ;; valid but semantically-wrong Rust.
      (define (vminstr->builder-call v)
        (let ([op (vminstr-op v)]
              [args (vminstr-arg* v)])
          (cond
            [(string=? op "idx")
             (let* ([cached-pair (assoc "cached" args)]
                    [push-pair (assoc "pushPath" args)]
                    [path-pair (assoc "path" args)])
               (cond
                 [(not (and cached-pair push-pair path-pair)) #f]
                 [else
                  (let* ([indices (vm-path->indices (cdr path-pair))]
                         [push-path (cdr push-pair)]
                         [path-val (cdr path-pair)]
                         [runtime-key
                          (and (list? path-val)
                               (pair? path-val)
                               (null? (cdr path-val))
                               (vm-rust-expr? (car path-val))
                               (car path-val))])
                    ;; A11: emit one `.idx_at_index(N, push)` per index.
                    ;; Single- and multi-index paths share the loop — the
                    ;; n=1 case (Counter et al. at top-level) is just the
                    ;; specialisation. did.compact's recordUpdate writes
                    ;; under a nested ledger struct (path `(1 6)`) needs
                    ;; n>1; A8 already did this for vminstr->gather-builder-call,
                    ;; this mirrors it for the Verify pipeline.
                    ;;
                    ;; A16: handle runtime-keyed paths (single vm-rust-expr
                    ;; in a 1-element list) for Map.insert / Map.lookup
                    ;; key indexing.
                    (cond
                      [runtime-key
                       (format "            .idx(~a, ~a, vec![midnight_compact_runtime::Key::Value(midnight_compact_runtime::AlignedValue::from(~a))])\n"
                               (if (cdr cached-pair) "true" "false")
                               (if push-path "true" "false")
                               (vm-rust-expr-text runtime-key))]
                      [(and indices (pair? indices))
                       (let loop ([is indices] [acc ""])
                         (cond
                           [(null? is) acc]
                           [else
                            (loop (cdr is)
                                  (string-append
                                    acc
                                    (format "            .idx_at_index(~au8, ~a)\n"
                                            (car is)
                                            (if push-path "true" "false"))))]))]
                      [else #f]))]))]
            [(string=? op "addi")
             (let ([imm-pair (assoc "immediate" args)])
               (cond
                 [(not imm-pair) #f]
                 [else
                  (let* ([imm (cdr imm-pair)]
                         [n (vm-immediate->int imm)])
                    (cond
                      ;; Literal integer (counter's `round.increment(1)`
                      ;; lowers the `1` to a Scheme exact int that
                      ;; vm-immediate->int unwraps directly).
                      [n (format "            .addi(~a)\n" n)]
                      ;; A4: when a multi-stmt body lowers an integer
                      ;; literal arg through a const-binding (Prod-14's
                      ;; `let tmp = 1u16; ops.increment(tmp);` shape),
                      ;; the `addi` immediate carries a vm-rust-expr
                      ;; whose `.text` is the rendered Rust reference
                      ;; (e.g. "tmp.clone()"). Emit it as a Rust
                      ;; expression cast to u32 (the addi parameter
                      ;; width per runtime-rs/src/op_builder.rs:61).
                      ;; The const-binding's source type might be
                      ;; u8/u16/u32/u64 depending on the literal's
                      ;; declared Uint<N> bound; `as u32` is the
                      ;; conservative target. Counter.increment's amount
                      ;; is typed Uint<32> on the runtime side, so this
                      ;; cast never silently truncates valid inputs.
                      [(vm-rust-expr? imm)
                       (format "            .addi(~a as u32)\n"
                               (vm-rust-expr-text imm))]
                      ;; Some `(VMvalue->int <vm-rust-expr>)` wrapping is
                      ;; also possible if the immediate went through the
                      ;; full vm-value path. Unwrap once.
                      [(and (VMop? imm)
                            (VMop-case imm
                              [(VMvalue->int x) (vm-rust-expr? x)]
                              [else #f]))
                       (let ([inner
                              (VMop-case imm
                                [(VMvalue->int x) x]
                                [else #f])])
                         (format "            .addi(~a as u32)\n"
                                 (vm-rust-expr-text inner)))]
                      [else #f]))]))]
            [(string=? op "rem")
             ;; A13: Set.remove / Map.remove vm-code op. Renders as
             ;; `.rem(<cached>)` on OpProgramVerify; the runtime
             ;; builder method was added alongside this change.
             (let ([cached-pair (assoc "cached" args)])
               (cond
                 [(not cached-pair) #f]
                 [else
                  (format "            .rem(~a)\n"
                          (if (cdr cached-pair) "true" "false"))]))]
            [(string=? op "ins")
             (let ([cached-pair (assoc "cached" args)]
                   [n-pair (assoc "n" args)])
               (cond
                 [(not (and cached-pair n-pair)) #f]
                 [else
                  (let ([n (cdr n-pair)])
                    (and (integer? n) (exact? n)
                         (format "            .ins(~a, ~a)\n"
                                 (if (cdr cached-pair) "true" "false")
                                 n)))]))]
            [(string=? op "push")
             (let ([storage-pair (assoc "storage" args)]
                   [value-pair (assoc "value" args)])
               (cond
                 [(not (and storage-pair value-pair)) #f]
                 [else
                  (let ([rust-val (vm-value->rust-state-value (cdr value-pair))])
                    (and rust-val
                         (format "            .push(~a, ~a)\n"
                                 (if (cdr storage-pair) "true" "false")
                                 rust-val)))]))]
            [(string=? op "dup")
             (let ([n-pair (assoc "n" args)])
               (cond
                 [(not n-pair) #f]
                 [else
                  (let ([n (cdr n-pair)])
                    (and (integer? n) (exact? n)
                         (format "            .dup(~a)\n" n)))]))]
            [(string=? op "root")
             (cond
               [(null? args) "            .root()\n"]
               [else #f])]
            ;; A21: branching / stack-shuffling ops emitted by the
            ;; bounded-index MerkleTree / HistoricMerkleTree
            ;; `insert*Index*` vm-code. The sequence is a compile-time
            ;; `max(old_first_free, index + 1)` realised on the VM stack:
            ;;
            ;;     (dup 1) (dup 1) (lt) (branch 2) (pop) (jmp 2)
            ;;     (swap 0) (pop)
            ;;
            ;; — push two copies of the candidates, compare, then either
            ;; drop the old counter (taken branch) or swap-then-drop the
            ;; new one (fallthrough). The Rust builder mirrors the ops 1:1;
            ;; `query_for_verify` walks them with the same VM semantics as
            ;; the TS reference, so byte-parity is preserved without
            ;; recovering structured control flow.
            [(string=? op "lt")
             (cond
               [(null? args) "            .lt()\n"]
               [else #f])]
            [(string=? op "pop")
             (cond
               [(null? args) "            .pop()\n"]
               [else #f])]
            [(string=? op "branch")
             (let ([skip-pair (assoc "skip" args)])
               (cond
                 [(not skip-pair) #f]
                 [else
                  (let ([skip (cdr skip-pair)])
                    (and (integer? skip) (exact? skip)
                         (format "            .branch(~a)\n" skip)))]))]
            [(string=? op "jmp")
             (let ([skip-pair (assoc "skip" args)])
               (cond
                 [(not skip-pair) #f]
                 [else
                  (let ([skip (cdr skip-pair)])
                    (and (integer? skip) (exact? skip)
                         (format "            .jmp(~a)\n" skip)))]))]
            [(string=? op "swap")
             (let ([n-pair (assoc "n" args)])
               (cond
                 [(not n-pair) #f]
                 [else
                  (let ([n (cdr n-pair)])
                    (and (integer? n) (exact? n)
                         (format "            .swap(~a)\n" n)))]))]
            [else #f])))

      ;; vminstr->gather-builder-call: like vminstr->builder-call but for
      ;; OpProgramGather chains emitted inline by emit-ledger-read-expr
      ;; (ADT `read` ops with args, e.g. Set.member, HistoricMerkleTree
      ;; .checkRoot, Map.member). Uses 16-space indentation to match the
      ;; existing read-expr block template and emits the additional ops
      ;; that read vm-code uses but write vm-code doesn't: `popeq` (no
      ;; result value in Gather mode), `member`, and `eq`. The `popeq`
      ;; arg layout matches Op::Popeq's `(cached, ())` Gather signature
      ;; via OpProgramGather::popeq(cached). Returns #f for ops we don't
      ;; know how to render so the caller falls back to the no-arg
      ;; hardcoded template.
      (define (vminstr->gather-builder-call v)
        (let ([op (vminstr-op v)]
              [args (vminstr-arg* v)])
          (cond
            [(string=? op "idx")
             (let* ([cached-pair (assoc "cached" args)]
                    [push-pair (assoc "pushPath" args)]
                    [path-pair (assoc "path" args)])
               (cond
                 [(not (and cached-pair push-pair path-pair)) #f]
                 [else
                  (let* ([indices (vm-path->indices (cdr path-pair))]
                         [push-path (cdr push-pair)]
                         [path-val (cdr path-pair)]
                         ;; A16: runtime-keyed idx — path is a 1-element
                         ;; list containing a vm-rust-expr (Map.lookup's
                         ;; second idx with `disclosed_method_id.clone()`).
                         [runtime-key
                          (and (list? path-val)
                               (pair? path-val)
                               (null? (cdr path-val))
                               (vm-rust-expr? (car path-val))
                               (car path-val))])
                    ;; A8: emit one `.idx_at_index(N, push)` per index.
                    ;; Single- and multi-index paths share the loop; the
                    ;; one-index case is just the n=1 specialisation
                    ;; (Map<K,V>.member etc. need n>1).
                    ;;
                    ;; A16: also handle runtime-keyed paths — emit
                    ;; `.idx(cached, push, vec![Key::Value(AlignedValue::from(<expr>))])`.
                    (cond
                      [runtime-key
                       (format "                .idx(~a, ~a, vec![midnight_compact_runtime::Key::Value(midnight_compact_runtime::AlignedValue::from(~a))])\n"
                               (if (cdr cached-pair) "true" "false")
                               (if push-path "true" "false")
                               (vm-rust-expr-text runtime-key))]
                      [(and indices (pair? indices))
                       (let loop ([is indices] [acc ""])
                         (cond
                           [(null? is) acc]
                           [else
                            (loop (cdr is)
                                  (string-append
                                    acc
                                    (format "                .idx_at_index(~au8, ~a)\n"
                                            (car is)
                                            (if push-path "true" "false"))))]))]
                      [else #f]))]))]
            [(string=? op "dup")
             (let ([n-pair (assoc "n" args)])
               (cond
                 [(not n-pair) #f]
                 [else
                  (let ([n (cdr n-pair)])
                    (and (integer? n) (exact? n)
                         (format "                .dup(~a)\n" n)))]))]
            [(string=? op "push")
             (let ([storage-pair (assoc "storage" args)]
                   [value-pair (assoc "value" args)])
               (cond
                 [(not (and storage-pair value-pair)) #f]
                 [else
                  (let ([rust-val (vm-value->rust-state-value (cdr value-pair))])
                    (and rust-val
                         (format "                .push(~a, ~a)\n"
                                 (if (cdr storage-pair) "true" "false")
                                 rust-val)))]))]
            [(string=? op "member")
             (cond
               [(null? args) "                .member()\n"]
               [else #f])]
            [(string=? op "eq")
             (cond
               [(null? args) "                .eq()\n"]
               [else #f])]
            [(string=? op "root")
             (cond
               [(null? args) "                .root()\n"]
               [else #f])]
            [(string=? op "size")
             ;; A20: `size` replaces the top-of-stack container with a
             ;; Cell holding its element count. Emitted by Set.size /
             ;; Set.isEmpty / Map.size / Map.isEmpty.
             (cond
               [(null? args) "                .size()\n"]
               [else #f])]
            [(string=? op "type")
             ;; A20: `type` replaces the top-of-stack StateValue with a
             ;; Cell holding its shape tag. Emitted by List.isEmpty to
             ;; detect the Null sentinel at the head of an empty list.
             (cond
               [(null? args) "                .type_()\n"]
               [else #f])]
            [(string=? op "popeq")
             (let ([cached-pair (assoc "cached" args)])
               (cond
                 [(not cached-pair) #f]
                 [else
                  (format "                .popeq(~a)\n"
                          (if (cdr cached-pair) "true" "false"))]))]
            [else #f])))
