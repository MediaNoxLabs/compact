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

      ;; -----------------------------------------------------------------
      ;; Prefix-instantiation disambiguation tables.
      ;;
      ;; `import M<...> prefix P_` instantiates generic circuits/structs.
      ;; The frontend keeps a BASE id-sym (e.g. `assertValidRequestMessage`)
      ;; and disambiguates instantiations only via `id-uniq`; the prefix
      ;; lives in the export-name. Two instantiations therefore share an
      ;; id-sym / struct-name, so naming Rust identifiers by bare
      ;; `(camel->snake (id-sym ...))` alone produces duplicate
      ;; `fn assert_valid_request_message` (E0428) and a silently-deduped
      ;; wrong-fielded `pub struct RequestMessage` (E0609).
      ;;
      ;; `build-id-rust-name-ht` and `build-struct-rust-name-ht` (called
      ;; once in the Program pass) produce two tables that the emitters
      ;; consult via `id->rust-name` / `struct-rust-name`:
      ;;   - exported circuit ids  → prefixed export-name (camel->snake'd)
      ;;   - non-exported circuit ids whose id-sym collides (shared by >1
      ;;     distinct circuit id) → `<snake>_<id-uniq>`
      ;;   - non-colliding circuit ids → bare `<snake>` (snapshot-preserving)
      ;;   - tstruct type nodes whose struct-name is shared by >1 distinct
      ;;     field fingerprint → `Name`, `Name_1`, `Name_2`, ... (first
      ;;     fingerprint encountered keeps the bare name so the surviving
      ;;     upstream export-typedef renders byte-identically)
      ;;   - non-colliding tstruct nodes → bare struct-name
      ;; -----------------------------------------------------------------

      ;; build-id-rust-name-ht: eq?-hashtable id -> disambiguated Rust name
      ;; symbol. `export-alist` is the Program's `((export-name . id) ...)`;
      ;; `circuit*` is the list of user circuit Program-Elements.
      (define (build-id-rust-name-ht export-alist circuit*)
        (let ([ht (make-eq-hashtable)]
              [export-id-ht (make-eq-hashtable)]   ; id -> export-name symbol
              [sym-freq (make-hashtable symbol-hash eq?)])
          ;; Map each exported circuit id to (one of) its export-name(s).
          (for-each
            (lambda (a)
              (let ([en (car a)] [id (cdr a)])
                (when (id-exported? id)
                  (unless (eq-hashtable-ref export-id-ht id #f)
                    (eq-hashtable-set! export-id-ht id en)))))
            export-alist)
          ;; Frequency of each id-sym across all circuit ids.
          (for-each
            (lambda (c)
              (let ([sym (id-sym (circuit-function-name c))])
                (hashtable-update! sym-freq sym
                  (lambda (n) (+ n 1)) 0)))
            circuit*)
          ;; Populate the name table.
          (for-each
            (lambda (c)
              (let* ([id (circuit-function-name c)]
                     [sym (id-sym id)]
                     [base (camel->snake sym)]
                     [en (eq-hashtable-ref export-id-ht id #f)]
                     [freq (hashtable-ref sym-freq sym 0)])
                (cond
                  [en
                   ;; Exported: use the prefixed export-name. For normal
                   ;; circuits export-name == id-sym, so this preserves
                   ;; existing snapshots.
                   (eq-hashtable-set! ht id (camel->snake en))]
                  [(> freq 1)
                   ;; Non-exported colliding id: append _<uniq>.
                   (eq-hashtable-set! ht id
                     (string->symbol (format "~a_~a" base (id-uniq id))))]
                  [else
                   ;; Non-colliding: bare name.
                   (eq-hashtable-set! ht id base)])))
            circuit*)
          ht))

      ;; build-struct-rust-name-ht: eq?-hashtable tstruct Type NODE ->
      ;; disambiguated Rust name symbol. Walks every tstruct type node
      ;; reachable from the export-typedefs and circuit/witness/native/
      ;; ledger sigs, fingerprints each (struct-name + rendered field
      ;; types, computed while the table is still empty so type-rust
      ;; renders bare struct-names), and — for struct-names shared by >1
      ;; distinct fingerprint — assigns `Name`, `Name_1`, ... in first-
      ;; fingerprint-encounter order (export-typedef nodes are walked
      ;; first so the surviving upstream decl keeps the bare name).
      (define (build-struct-rust-name-ht tdefn* circuit* witness* native*
                                          ledger*)
        (let ([node->fp (make-eq-hashtable)]
              [name->entries (make-hashtable symbol-hash eq?)])
          ;; Record a tstruct node + its fingerprint. Idempotent on eq?
          ;; (a node shared across sigs is recorded once). Returns the
          ;; node so callers can recurse into its field types once.
          (define (record-tstruct! type struct-name)
            (unless (eq-hashtable-ref node->fp type #f)
              (let ([fp (tstruct-fingerprint type)])
                (eq-hashtable-set! node->fp type fp)
                (hashtable-update! name->entries struct-name
                  (lambda (lst) (cons (cons fp type) lst)) '())))
            type)
          ;; Recursively walk a Type, recording every tstruct node and
          ;; recursing into field / element / alias-underlying types so
          ;; nested user structs get entered too.
          (define (collect-type type)
            (nanopass-case (Ltypescript Type) type
              [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
               (record-tstruct! type struct-name)
               (for-each collect-type type*)]
              [(tenum ,src ,enum-name ,elt-name ,elt-name* ...) (void)]
              [(tvector ,src ,len ,type) (collect-type type)]
              [(ttuple ,src ,type* ...) (for-each collect-type type*)]
              [(talias ,src ,nominal? ,type-name ,type) (collect-type type)]
              [(tcontract ,src ,contract-name (,elt-name* ,pure-dcl* (,type** ...) ,type*) ...)
               (for-each (lambda (ts*) (for-each collect-type ts*)) type**)]
              [else (void)]))
          (define (collect-arg arg)
            (nanopass-case (Ltypescript Argument) arg
              [(,var-name ,type) (collect-type type)]))
          ;; Walk export-typedefs first (so their fingerprint wins the
          ;; bare name), then every sig-bearing pelt.
          (for-each
            (lambda (td)
              (nanopass-case (Ltypescript Program-Element) td
                [(export-typedef ,src ,type-name (,tvar-name* ...) ,type)
                 (collect-type type)]
                [else (void)]))
            tdefn*)
          (for-each
            (lambda (c)
              (nanopass-case (Ltypescript Program-Element) c
                [(circuit ,src ,function-name (,arg* ...) ,type ,stmt)
                 (for-each collect-arg arg*)
                 (collect-type type)]
                [else (void)]))
            circuit*)
          (for-each
            (lambda (w)
              (nanopass-case (Ltypescript Program-Element) w
                [(witness ,src ,function-name (,arg* ...) ,type)
                 (for-each collect-arg arg*)
                 (collect-type type)]
                [else (void)]))
            witness*)
          (for-each
            (lambda (n)
              (nanopass-case (Ltypescript Program-Element) n
                [(native ,src ,function-name ,native-entry (,arg* ...) ,type)
                 (for-each collect-arg arg*)
                 (collect-type type)]
                [else (void)]))
            native*)
          (for-each
            (lambda (lf)
              (nanopass-case (Ltypescript Program-Element) lf
                [(public-ledger-declaration ,pl-array ,lconstructor)
                 (for-each
                   (lambda (pb)
                     (nanopass-case (Ltypescript Public-Ledger-Binding) pb
                       [(,src ,ledger-field-name (,path-index* ...) ,type)
                        (collect-type type)]
                       [else (void)]))
                   (pl-array->public-bindings pl-array))]
                [else (void)]))
            ledger*)
          ;; Assign names: for each struct-name, group its entries by
          ;; fingerprint (equal?), preserving first-encounter order. A
          ;; single fingerprint group → bare name; >1 → Name, Name_1, ...
          (let ([ht (make-eq-hashtable)])
            (let-values ([(names entries-vec) (hashtable-entries name->entries)])
            (vector-for-each
              (lambda (name entries)
                (let ([groups (let group ([es (reverse entries)] [acc '()])
                                (cond
                                  [(null? es) acc]
                                  [else
                                   (let ([fp (caar es)] [node (cdar es)])
                                     (let ([existing (ormap (lambda (g) (and (equal? (car g) fp) g)) acc)])
                                       (cond
                                         [existing
                                          (set-cdr! existing (cons node (cdr existing)))
                                          (group (cdr es) acc)]
                                         [else
                                          (group (cdr es)
                                            (cons (cons fp (list node)) acc))])))]))])
                  (cond
                    [(null? (cdr groups))
                     ;; single fingerprint: bare name for every node
                     (for-each
                       (lambda (node) (eq-hashtable-set! ht node name))
                       (cdr (car groups)))]
                    [else
                     ;; colliding: bare, _1, _2, ... per group (groups are
                     ;; in first-encounter order thanks to the reverse+cons).
                     (let loop ([gs groups] [idx 0])
                       (unless (null? gs)
                         (let ([rname (if (= idx 0)
                                          name
                                          (string->symbol (format "~a_~a" name idx)))])
                           (for-each
                             (lambda (node) (eq-hashtable-set! ht node rname))
                             (cdr (car gs)))
                           (loop (cdr gs) (+ idx 1)))))])))
              names
              entries-vec))
            ht)))