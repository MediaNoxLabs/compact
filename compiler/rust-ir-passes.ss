#!chezscheme

;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0
;;; Licensed under the Apache License, Version 2.0 (the "License");
;;; you may not use this file except in compliance with the License.
;;; You may obtain a copy of the License at
;;;
;;; http://www.apache.org/licenses/LICENSE-2.0
;;;
;;; Unless required by applicable law or agreed to in writing, software
;;; distributed under the License is distributed on an "AS IS" BASIS,
;;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;;; See the License for the specific language governing permissions and
;;; limitations under the License.

;;; The Rust target starts from Lnodisclose, before any TypeScript-specific
;;; pass. Scheme owns Compact semantics and emits a closed JSON domain model;
;;; compact-rust-backend constructs and prints Rust syntax from that model.

(library (rust-ir-passes)
  (export rust-ir-passes)
  (import (except (chezscheme) errorf)
          (utils)
          (nanopass)
          (json)
          (langs)
          (pass-helpers))

  (define-pass emit-rust-ir : Lnodisclose (ir) -> Lnodisclose ()
    (definitions
      (define (object . fields) fields)

      (define (kind name)
        (object (cons "kind" name)))

      (define (type-ir ty owner-src)
        (nanopass-case (Lnodisclose Type) ty
          [(tboolean ,src) (kind "boolean")]
          [(tfield ,src) (kind "field")]
          [(tbytes ,src ,len)
           (object (cons "kind" "bytes")
                   (cons "length" len))]
          [(tstruct ,src ,struct-name (,elt-name* ,type*) ...)
           (object (cons "kind" "struct")
                   (cons "name" (symbol->string struct-name))
                   (cons "fields"
                         (list->vector
                           (map (lambda (name ty)
                                  (object (cons "name" (symbol->string name))
                                          (cons "ty" (type-ir ty owner-src))))
                                elt-name* type*))))]
          [(tenum ,src ,enum-name ,elt-name ,elt-name* ...)
           (object (cons "kind" "enum")
                   (cons "name" (symbol->string enum-name))
                   (cons "variants"
                         (list->vector (map symbol->string (cons elt-name elt-name*)))))]
          [(tunsigned ,src ,nat)
           (object (cons "kind" "unsigned")
                   (cons "max" (number->string nat)))]
          [(ttuple ,src ,type* ...)
           (if (null? type*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements" (list->vector (map (lambda (ty) (type-ir ty owner-src)) type*)))))]
          [(tvector ,src ,len ,type)
           (object (cons "kind" "vector")
                   (cons "length" len)
                   (cons "element" (type-ir type owner-src)))]
          [(talias ,src ,nominal? ,type-name ,type)
           (type-ir type owner-src)]
          [else (source-errorf owner-src "Rust backend does not yet support this Compact type")]))

      (define (tuple-argument-ir arg owner-src)
        (nanopass-case (Lnodisclose Tuple-Argument) arg
          [(single ,src ,expr) (expression-ir expr owner-src)]
          [else (source-errorf owner-src "Rust backend does not yet support tuple spreads")]))

      (define (expression-ir expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (expression-ir expr owner-src)]
          [(var-ref ,src ,var-name)
           (object (cons "kind" "parameter")
                   (cons "name" (symbol->string (id-sym var-name))))]
          [(quote ,src ,datum)
           (if (boolean? datum)
               (object (cons "kind" "boolean") (cons "value" datum))
               (source-errorf src "Rust backend does not yet support this literal"))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements"
                             (list->vector
                               (map (lambda (arg) (tuple-argument-ir arg owner-src)) tuple-arg*)))))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (when mbits
             (source-errorf src "Rust backend does not yet support bounded unsigned arithmetic"))
           (object (cons "kind" "add")
                   (cons "left" (expression-ir expr1 owner-src))
                   (cons "right" (expression-ir expr2 owner-src)))]
          [else (source-errorf owner-src "Rust backend does not yet support this circuit expression")]))

      (define (argument-ir arg owner-src)
        (nanopass-case (Lnodisclose Argument) arg
          [(,var-name ,type)
           (object (cons "name" (symbol->string (id-sym var-name)))
                   (cons "ty" (type-ir type owner-src)))]))

      (define (exported-names function-name export-alist)
        (fold-right
          (lambda (entry names)
            (if (eq? (cdr entry) function-name)
                (cons (symbol->string (car entry)) names)
                names))
          '()
          export-alist))

      (define (circuit-ir pelt export-alist circuits)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(circuit ,src ,function-name (,arg* ...) ,type ,expr)
           (let ([names (exported-names function-name export-alist)])
             (if (null? names)
                 circuits
                 (if (id-pure? function-name)
                   (append
                     (map
                       (lambda (name)
                         (object (cons "name" name)
                                 (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                                 (cons "result" (type-ir type src))
                                 (cons "body" (expression-ir expr src))))
                       names)
                     circuits)
                   circuits)))]
          [else circuits]))

      (define (ledger-binding-ir binding owner-src)
        (nanopass-case (Lnodisclose Public-Ledger-Array-Element) binding
          [(,src ,ledger-field-name (,path-index* ...) ,type)
           (unless (and (= (length path-index*) 1)
                        (< (car path-index*) 16))
             (source-errorf src "Rust backend supports root ledger fields only"))
           (nanopass-case (Lnodisclose Type) type
             [(tadt ,src^ ,adt-name ([,adt-formal* ,adt-arg*] ...) ,vm-expr (,adt-op* ...) (,adt-rt-op* ...))
              (cond
                [(eq? adt-name 'Counter)
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration" (kind "counter")))]
                [(and (eq? adt-name '__compact_Cell) (= (length adt-arg*) 1))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "cell")
                                       (cons "ty" (type-ir (car adt-arg*) src)))))]
                [(and (eq? adt-name 'Set) (= (length adt-arg*) 1))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "set")
                                       (cons "ty" (type-ir (car adt-arg*) src)))))]
                [(and (eq? adt-name 'List) (= (length adt-arg*) 1))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "list")
                                       (cons "ty" (type-ir (car adt-arg*) src)))))]
                [(and (eq? adt-name 'Map) (= (length adt-arg*) 2))
                 (object (cons "id" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-index*))
                         (cons "declaration"
                               (object (cons "kind" "map")
                                       (cons "key" (type-ir (car adt-arg*) src))
                                       (cons "value" (type-ir (cadr adt-arg*) src)))))]
                [else (source-errorf src "Rust backend does not yet support this ledger ADT")])]
             [else (source-errorf src "Rust backend does not yet support this ledger field type")])]
          [else (source-errorf owner-src "Rust backend does not yet support nested ledger fields")]))

      (define (ledger-fields-ir pelt fields owner-src)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(public-ledger-declaration ,pl-array ,lconstructor)
           (nanopass-case (Lnodisclose Public-Ledger-Array) pl-array
             [(public-ledger-array ,pl-array-elt* ...)
              (append (map (lambda (binding) (ledger-binding-ir binding owner-src)) pl-array-elt*) fields)])]
          [else fields]))

      (define (counter-amount-ir expr environment owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(quote ,src ,datum)
           (unless (and (integer? datum) (<= 0 datum 65535))
             (source-errorf src "Counter increment amount must fit Uint<16>"))
           (object (cons "kind" "literal") (cons "value" datum))]
          [(safe-cast ,src ,type ,type^ ,expr)
           (counter-amount-ir expr environment owner-src)]
          [(var-ref ,src ,var-name)
           (let ([entry (assq (id-sym var-name) environment)])
             (if entry
                 (cdr entry)
                 (source-errorf src "Rust backend cannot resolve Counter increment amount")))]
          [else (source-errorf owner-src "Rust backend supports literal or parameter Counter increments only")]))

      (define (state-action-ir expr owner-src environment)
        (nanopass-case (Lnodisclose Expression) expr
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (let ([environment^
                   (fold-left
                     (lambda (environment local value)
                       (nanopass-case (Lnodisclose Argument) local
                         [(,var-name ,type)
                          (cons (cons (id-sym var-name)
                                      (counter-amount-ir value environment src))
                                environment)]))
                     environment local* expr*)])
             (state-action-ir expr owner-src environment^))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           (unless (and (= (length path-elt*) 1)
                        (integer? (car path-elt*)))
             (source-errorf src "Rust backend supports root ledger paths only"))
           (nanopass-case (Lnodisclose ADT-Op) adt-op
             [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
              (cond
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'increment)
                      (= (length expr*) 1))
                 (object (cons "kind" "counter_increment")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "amount" (counter-amount-ir (car expr*) environment src)))]
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'decrement)
                      (= (length expr*) 1))
                 (object (cons "kind" "counter_decrement")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "amount" (counter-amount-ir (car expr*) environment src)))]
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "counter_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name '__compact_Cell)
                      (eq? ledger-op 'write)
                      (= (length expr*) 1))
                 (object (cons "kind" "cell_write")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'insert)
                      (= (length expr*) 1))
                 (object (cons "kind" "set_insert")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'remove)
                      (= (length expr*) 1))
                 (object (cons "kind" "set_remove")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "set_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'pushFront)
                      (= (length expr*) 1))
                 (object (cons "kind" "list_push_front")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'popFront)
                      (null? expr*))
                 (object (cons "kind" "list_pop_front")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "list_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'insert)
                      (= (length expr*) 2))
                 (object (cons "kind" "map_insert")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src))
                         (cons "value" (expression-ir (cadr expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'insertDefault)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_insert_default")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'remove)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_remove")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'resetToDefault)
                      (null? expr*))
                 (object (cons "kind" "map_reset")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [else (source-errorf src "Rust backend does not yet support this ledger operation")])])]
          [else (source-errorf owner-src "Rust backend does not yet support this state action")]))

      (define (stateful-body-ir expr src environment)
        (nanopass-case (Lnodisclose Expression) expr
          [(seq ,src1 ,expr* ... ,expr)
           (list->vector (map (lambda (action) (state-action-ir action src environment)) expr*))]
          [else (vector)]))

      (define (stateful-return-ir expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (stateful-return-ir expr src)]
          [(seq ,src ,expr* ... ,expr) (stateful-return-ir expr src)]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (source-errorf src "Rust backend does not yet support this stateful return value"))]
          [(public-ledger ,src ,ledger-field-name ,sugar? (,path-elt* ...) ,src^ ,adt-op ,expr* ...)
           (unless (and (= (length path-elt*) 1)
                        (integer? (car path-elt*)))
             (source-errorf src "Rust backend supports root ledger paths only"))
           (nanopass-case (Lnodisclose ADT-Op) adt-op
             [(,ledger-op ,op-class (,adt-name (,adt-formal* ,adt-arg*) ...) ((,var-name* ,type*) ...) ,type ,vm-code)
              (cond
                [(and (eq? adt-name '__compact_Cell)
                      (eq? ledger-op 'read)
                      (null? expr*))
                 (object (cons "kind" "cell_read")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Counter)
                      (eq? ledger-op 'read)
                      (null? expr*))
                 (object (cons "kind" "counter_read")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'member)
                      (= (length expr*) 1))
                 (object (cons "kind" "set_member")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "value" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'size)
                      (null? expr*))
                 (object (cons "kind" "set_size")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Set)
                      (eq? ledger-op 'isEmpty)
                      (null? expr*))
                 (object (cons "kind" "set_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'member)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_member")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'lookup)
                      (= (length expr*) 1))
                 (object (cons "kind" "map_lookup")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*))
                         (cons "key" (expression-ir (car expr*) src)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'size)
                      (null? expr*))
                 (object (cons "kind" "map_size")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'Map)
                      (eq? ledger-op 'isEmpty)
                      (null? expr*))
                 (object (cons "kind" "map_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'length)
                      (null? expr*))
                 (object (cons "kind" "list_length")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'isEmpty)
                      (null? expr*))
                 (object (cons "kind" "list_is_empty")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [(and (eq? adt-name 'List)
                      (eq? ledger-op 'head)
                      (null? expr*))
                 (object (cons "kind" "list_head")
                         (cons "field" (symbol->string (id-sym ledger-field-name)))
                         (cons "index" (car path-elt*)))]
                [else (source-errorf src "Rust backend does not yet support this ledger return operation")])])]
          [else (source-errorf owner-src "Rust backend does not yet support this stateful return value")]))

      (define (stateful-circuit-ir pelt export-alist circuits)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(circuit ,src ,function-name (,arg* ...) ,type ,expr)
           (if (id-pure? function-name)
               circuits
               (let ([names (exported-names function-name export-alist)])
                 (if (null? names)
                     circuits
                     (begin
                       (append
                         (map (lambda (name)
                                (object (cons "name" name)
                                        (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                                        (cons "result" (type-ir type src))
                                        (cons "return_value" (stateful-return-ir expr src))
                                        (cons "actions"
                                              (stateful-body-ir expr src
                                                (map (lambda (arg)
                                                       (nanopass-case (Lnodisclose Argument) arg
                                                         [(,var-name ,type)
                                                          (cons (id-sym var-name)
                                                                (object (cons "kind" "parameter")
                                                                        (cons "name" (symbol->string (id-sym var-name)))))]))
                                                     arg*)))))
                              names)
                         circuits)))))]
          [else circuits]))

      (define (check-supported-declaration pelt owner-src)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type)
           (source-errorf src "Rust backend does not yet support witnesses")]
          [(public-ledger-declaration ,pl-array ,lconstructor)
           (void)]
          [else (void)])))

    (Program : Program (ir) -> Program ()
      [(program ,src (,contract-name* ...) ((,export-name* ,name*) ...) ,pelt* ...)
       (for-each (lambda (pelt) (check-supported-declaration pelt src)) pelt*)
       (let ([export-alist (map cons export-name* name*)])
         (print-json
           (get-target-port 'rust.ir.json)
           (object (cons "schema_version" 3)
                   (cons "ledger_fields"
                         (list->vector
                           (fold-right (lambda (pelt fields) (ledger-fields-ir pelt fields src)) '() pelt*)))
                   (cons "circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (circuit-ir pelt export-alist circuits))
                             '()
                             pelt*)))
                   (cons "stateful_circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (stateful-circuit-ir pelt export-alist circuits))
                             '()
                             pelt*))))))
       ir]))

  (define-passes rust-ir-passes
    (emit-rust-ir Lnodisclose)))
