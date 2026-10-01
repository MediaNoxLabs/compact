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

      (define (maybe-nonnegative-integer-literal expr owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (maybe-nonnegative-integer-literal expr src)]
          [(quote ,src ,datum)
           (if (and (integer? datum) (<= 0 datum))
               datum
               (source-errorf src "Rust backend supports nonnegative numeric literals only"))]
          [else #f]))

      ;; Analysis inserts this guard for Uint subtraction. The native checked
      ;; subtraction performs the same comparison, so only this exact guard
      ;; may be folded into the arithmetic expression.
      (define (checked-unsigned-subtraction? statements tail)
        (and (= (length statements) 1)
             (nanopass-case (Lnodisclose Expression) tail
               [(- ,src ,mbits ,expr1 ,expr2)
                (and mbits
                     (nanopass-case (Lnodisclose Expression) (car statements)
                       [(assert ,src1 ,expr5 ,mesg)
                        (and (equal? mesg "result of subtraction would be negative")
                             (nanopass-case (Lnodisclose Expression) expr5
                               [(>= ,src2 ,bits ,expr3 ,expr4)
                                (and (= bits mbits)
                                     (equal? (expression-ir expr3 src)
                                             (expression-ir expr1 src))
                                     (equal? (expression-ir expr4 src)
                                             (expression-ir expr2 src)))]
                               [else #f]))]
                       [else #f]))]
               [else #f])))

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
          [(safe-cast ,src ,type ,type^ ,expr)
           (nanopass-case (Lnodisclose Type) type
             [(tfield ,src^)
              (let ([value (maybe-nonnegative-integer-literal expr src)])
                (if value
                    (object (cons "kind" "field_literal")
                            (cons "value" (number->string value)))
                    (source-errorf src "Rust backend does not yet support this Field cast")))]
             [(tunsigned ,src^ ,nat)
              (let ([value (maybe-nonnegative-integer-literal expr src)])
                (if value
                    (begin
                      (unless (<= value nat)
                        (source-errorf src "Rust backend Uint literal exceeds its maximum"))
                      (object (cons "kind" "unsigned_literal")
                              (cons "value" (number->string value))
                              (cons "max" (number->string nat))))
                    (object (cons "kind" "unsigned_cast")
                            (cons "max" (number->string nat))
                            (cons "value" (typed-expression-ir expr type^ src)))))]
             [else (source-errorf src "Rust backend does not yet support this cast")])]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements"
                             (list->vector
                               (map (lambda (arg) (tuple-argument-ir arg owner-src)) tuple-arg*)))))]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "if")
                   (cons "condition" (expression-ir expr0 src))
                   (cons "then" (expression-ir expr1 src))
                   (cons "otherwise" (expression-ir expr2 src)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (object (cons "kind" "let")
                   (cons "bindings"
                         (list->vector
                           (map (lambda (local value)
                                  (nanopass-case (Lnodisclose Argument) local
                                    [(,var-name ,type)
                                     (object (cons "name" (symbol->string (id-sym var-name)))
                                             (cons "ty" (type-ir type src))
                                             (cons "value" (expression-ir value src)))]))
                                local* expr*)))
                   (cons "body" (expression-ir expr src)))]
          [(seq ,src ,expr* ... ,expr)
           (if (checked-unsigned-subtraction? expr* expr)
               (expression-ir expr src)
               (source-errorf src "Rust backend does not yet support this circuit expression"))]
          [(call ,src ,function-name ,expr* ...)
           (let ([name (id-sym function-name)])
             (cond
               [(eq? name 'transientHash)
                (unless (= (length expr*) 1)
                  (source-errorf src "transientHash expects one argument"))
                (object (cons "kind" "transient_hash")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'transientCommit)
                (unless (= (length expr*) 2)
                  (source-errorf src "transientCommit expects two arguments"))
               (object (cons "kind" "transient_commit")
                        (cons "value" (expression-ir (car expr*) src))
                        (cons "opening" (expression-ir (cadr expr*) src)))]
               [(eq? name 'persistentHash)
                (unless (= (length expr*) 1)
                  (source-errorf src "persistentHash expects one argument"))
                (object (cons "kind" "persistent_hash")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'persistentCommit)
                (unless (= (length expr*) 2)
                  (source-errorf src "persistentCommit expects two arguments"))
                (object (cons "kind" "persistent_commit")
                        (cons "value" (expression-ir (car expr*) src))
                        (cons "opening" (expression-ir (cadr expr*) src)))]
               [(eq? name 'degradeToTransient)
                (unless (= (length expr*) 1)
                  (source-errorf src "degradeToTransient expects one argument"))
                (object (cons "kind" "degrade_to_transient")
                        (cons "value" (expression-ir (car expr*) src)))]
               [(eq? name 'upgradeFromTransient)
                (unless (= (length expr*) 1)
                  (source-errorf src "upgradeFromTransient expects one argument"))
                (object (cons "kind" "upgrade_from_transient")
                        (cons "value" (expression-ir (car expr*) src)))]
               [else
                (object (cons "kind" "call")
                        (cons "name" (symbol->string name))
                        (cons "arguments" (list->vector (map (lambda (arg) (expression-ir arg src)) expr*))))]))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_add")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src)))
               (object (cons "kind" "add")
                       (cons "left" (expression-ir expr1 owner-src))
                       (cons "right" (expression-ir expr2 owner-src))))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_subtract")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src)))
               (object (cons "kind" "subtract")
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src))))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_multiply")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src)))
               (object (cons "kind" "multiply")
                       (cons "left" (expression-ir expr1 src))
                       (cons "right" (expression-ir expr2 src))))]
          [else (source-errorf owner-src "Rust backend does not yet support this circuit expression")]))

      (define (typed-expression-ir expr expected-type owner-src)
        (nanopass-case (Lnodisclose Expression) expr
          [(return ,src ,expr) (typed-expression-ir expr expected-type src)]
          [(seq ,src ,expr* ... ,expr)
           (if (checked-unsigned-subtraction? expr* expr)
               (typed-expression-ir expr expected-type src)
               (source-errorf src "Rust backend does not yet support this circuit expression"))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (object (cons "kind" "unsigned_add")
                          (cons "max" (number->string nat))
                          (cons "left" (expression-ir expr1 src))
                          (cons "right" (expression-ir expr2 src)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (object (cons "kind" "unsigned_subtract")
                          (cons "max" (number->string nat))
                          (cons "left" (expression-ir expr1 src))
                          (cons "right" (expression-ir expr2 src)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (object (cons "kind" "unsigned_multiply")
                          (cons "max" (number->string nat))
                          (cons "left" (expression-ir expr1 src))
                          (cons "right" (expression-ir expr2 src)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [(quote ,src ,datum)
           (if (and (integer? datum) (<= 0 datum))
               (nanopass-case (Lnodisclose Type) expected-type
                 [(tunsigned ,src^ ,nat)
                  (unless (<= datum nat)
                    (source-errorf src "Rust backend Uint literal exceeds its maximum"))
                  (object (cons "kind" "unsigned_literal")
                          (cons "value" (number->string datum))
                          (cons "max" (number->string nat)))]
                 [(tfield ,src^)
                  (object (cons "kind" "field_literal")
                          (cons "value" (number->string datum)))]
                 [else (expression-ir expr owner-src)])
               (expression-ir expr owner-src))]
          [else (expression-ir expr owner-src)]))

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

      (define (witness-declaration-ir pelt declarations)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type)
           (cons (object (cons "name" (symbol->string (id-sym function-name)))
                         (cons "parameters" (list->vector (map (lambda (arg) (argument-ir arg src)) arg*)))
                         (cons "result" (type-ir type src)))
                 declarations)]
          [else declarations]))

      (define (witness-id-table pelt*)
        (let ([table (make-eq-hashtable)])
          (for-each
            (lambda (pelt)
              (nanopass-case (Lnodisclose Program-Element) pelt
                [(witness ,src ,function-name (,arg* ...) ,type)
                 (eq-hashtable-set! table function-name #t)]
                [else (void)]))
            pelt*)
          table))

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
                                 (cons "body" (typed-expression-ir expr type src))))
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

      (define (state-action-ir expr owner-src environment witness-ids)
        (nanopass-case (Lnodisclose Expression) expr
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (let ([environment^
                   (fold-left
                     (lambda (environment local value)
                       (nanopass-case (Lnodisclose Argument) local
                         [(,var-name ,type)
                          (cons (cons (id-sym var-name)
                                      (object (cons "kind" "parameter")
                                              (cons "name" (symbol->string (id-sym var-name)))))
                                environment)]))
                     environment local* expr*)])
             (object (cons "kind" "let")
                     (cons "bindings"
                           (list->vector
                             (map (lambda (local value)
                                    (nanopass-case (Lnodisclose Argument) local
                                      [(,var-name ,type)
                                       (object (cons "name" (symbol->string (id-sym var-name)))
                                               (cons "ty" (type-ir type src))
                                               (cons "value" (stateful-expression-ir value src witness-ids)))]))
                                  local* expr*)))
                     (cons "action" (state-action-ir expr owner-src environment^ witness-ids))))]
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

      (define (stateful-body-ir expr src environment witness-ids)
        (nanopass-case (Lnodisclose Expression) expr
          [(seq ,src1 ,expr* ... ,expr)
           (list->vector (map (lambda (action) (state-action-ir action src environment witness-ids)) expr*))]
          [else (vector)]))

      ;; Stateful expressions keep witness calls explicit so Rust can evaluate
      ;; them in order and append each private transcript value exactly once.
      (define (stateful-expression-ir value-expr owner-src witness-ids)
        (nanopass-case (Lnodisclose Expression) value-expr
          [(return ,src ,expr) (stateful-expression-ir expr src witness-ids)]
          [(call ,src ,function-name ,expr* ...)
           (let ([name (id-sym function-name)])
             (cond
               [(eq-hashtable-ref witness-ids function-name #f)
                (object (cons "kind" "witness_call")
                        (cons "name" (symbol->string name))
                        (cons "arguments" (list->vector (map (lambda (arg) (stateful-expression-ir arg src witness-ids)) expr*))))]
               [(memq name '(transientHash persistentHash degradeToTransient upgradeFromTransient))
                (unless (= (length expr*) 1)
                  (source-errorf src "Rust backend native expects one argument"))
                (object (cons "kind" (case name
                                        [(transientHash) "transient_hash"]
                                        [(persistentHash) "persistent_hash"]
                                        [(degradeToTransient) "degrade_to_transient"]
                                        [else "upgrade_from_transient"]))
                        (cons "value" (stateful-expression-ir (car expr*) src witness-ids)))]
               [(memq name '(transientCommit persistentCommit))
                (unless (= (length expr*) 2)
                  (source-errorf src "Rust backend native expects two arguments"))
                (object (cons "kind" (if (eq? name 'transientCommit)
                                          "transient_commit"
                                          "persistent_commit"))
                        (cons "value" (stateful-expression-ir (car expr*) src witness-ids))
                        (cons "opening" (stateful-expression-ir (cadr expr*) src witness-ids)))]
               [else (expression-ir value-expr owner-src)]))]
          [(safe-cast ,src ,type ,type^ ,expr)
           (nanopass-case (Lnodisclose Type) type
             [(tunsigned ,src^ ,nat)
              (if (maybe-nonnegative-integer-literal expr src)
                  (expression-ir value-expr owner-src)
                  (object (cons "kind" "unsigned_cast")
                          (cons "max" (number->string nat))
                          (cons "value" (stateful-expression-ir expr src witness-ids))))]
             [else (expression-ir value-expr owner-src)])]
          [(downcast-unsigned ,src ,nat? ,nat ,expr)
           (unless nat?
             (source-errorf src "Rust backend does not yet support Field-to-Uint downcasts"))
           (object (cons "kind" "unsigned_cast")
                   (cons "max" (number->string nat))
                   (cons "value" (stateful-expression-ir expr src witness-ids)))]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "if")
                   (cons "condition" (stateful-expression-ir expr0 src witness-ids))
                   (cons "then" (stateful-expression-ir expr1 src witness-ids))
                   (cons "otherwise" (stateful-expression-ir expr2 src witness-ids)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (object (cons "kind" "let")
                   (cons "bindings"
                         (list->vector
                           (map (lambda (local value)
                                  (nanopass-case (Lnodisclose Argument) local
                                    [(,var-name ,type)
                                     (object (cons "name" (symbol->string (id-sym var-name)))
                                             (cons "ty" (type-ir type src))
                                             (cons "value" (stateful-expression-ir value src witness-ids)))]))
                                local* expr*)))
                   (cons "body" (stateful-expression-ir expr src witness-ids)))]
          [(seq ,src ,expr* ... ,expr)
           (if (checked-unsigned-subtraction? expr* expr)
               (stateful-expression-ir expr src witness-ids)
               (source-errorf src "Rust backend does not yet support this stateful expression sequence"))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "tuple")
                       (cons "elements"
                             (list->vector
                               (map (lambda (arg)
                                      (nanopass-case (Lnodisclose Tuple-Argument) arg
                                        [(single ,src ,expr)
                                         (stateful-expression-ir expr src witness-ids)]
                                        [else (source-errorf src "Rust backend does not yet support tuple spreads")]))
                                    tuple-arg*)))))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_add")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids)))
               (object (cons "kind" "add")
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids))))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_subtract")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids)))
               (object (cons "kind" "subtract")
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids))))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (if mbits
               (object (cons "kind" "unsigned_multiply")
                       (cons "max" (number->string (- (expt 2 mbits) 1)))
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids)))
               (object (cons "kind" "multiply")
                       (cons "left" (stateful-expression-ir expr1 src witness-ids))
                       (cons "right" (stateful-expression-ir expr2 src witness-ids))))]
          [else (expression-ir value-expr owner-src)]))

      (define (stateful-return-ir return-expr owner-src witness-ids)
        (nanopass-case (Lnodisclose Expression) return-expr
          [(return ,src ,expr) (stateful-return-ir expr src witness-ids)]
          [(seq ,src ,expr* ... ,expr) (stateful-return-ir expr src witness-ids)]
          [(call ,src ,function-name ,expr* ...)
           (if (or (eq-hashtable-ref witness-ids function-name #f)
                   (memq (id-sym function-name)
                         '(transientHash transientCommit persistentHash persistentCommit
                           degradeToTransient upgradeFromTransient)))
               (object (cons "kind" "expression")
                       (cons "value" (stateful-expression-ir return-expr src witness-ids)))
               (source-errorf src "Rust backend does not yet support this stateful call"))]
          [(tuple ,src ,tuple-arg* ...)
           (if (null? tuple-arg*)
               (kind "unit")
               (object (cons "kind" "expression")
                       (cons "value" (stateful-expression-ir return-expr src witness-ids))))]
          [(if ,src ,expr0 ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(let* ,src ([,local* ,expr*] ...) ,expr)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(safe-cast ,src ,type ,type^ ,expr)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(downcast-unsigned ,src ,nat? ,nat ,expr)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(+ ,src ,mbits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(- ,src ,mbits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
          [(* ,src ,mbits ,expr1 ,expr2)
           (object (cons "kind" "expression")
                   (cons "value" (stateful-expression-ir return-expr src witness-ids)))]
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

      (define (stateful-circuit-ir pelt export-alist witness-ids circuits)
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
                                        (cons "return_value" (stateful-return-ir expr src witness-ids))
                                        (cons "actions"
                                              (stateful-body-ir expr src
                                                (map (lambda (arg)
                                                       (nanopass-case (Lnodisclose Argument) arg
                                                         [(,var-name ,type)
                                                          (cons (id-sym var-name)
                                                                (object (cons "kind" "parameter")
                                                                        (cons "name" (symbol->string (id-sym var-name)))))]))
                                                     arg*) witness-ids))))
                              names)
                         circuits)))))]
          [else circuits]))

      (define (check-supported-declaration pelt owner-src)
        (nanopass-case (Lnodisclose Program-Element) pelt
          [(witness ,src ,function-name (,arg* ...) ,type)
           (void)]
          [(public-ledger-declaration ,pl-array ,lconstructor)
           (void)]
          [else (void)])))

    (Program : Program (ir) -> Program ()
      [(program ,src (,contract-name* ...) ((,export-name* ,name*) ...) ,pelt* ...)
       (for-each (lambda (pelt) (check-supported-declaration pelt src)) pelt*)
       (let ([export-alist (map cons export-name* name*)]
             [witness-ids (witness-id-table pelt*)])
         (print-json
           (get-target-port 'rust.ir.json)
           (object (cons "schema_version" 4)
                   (cons "ledger_fields"
                         (list->vector
                           (fold-right (lambda (pelt fields) (ledger-fields-ir pelt fields src)) '() pelt*)))
                   (cons "witnesses"
                         (list->vector
                           (fold-right witness-declaration-ir '() pelt*)))
                   (cons "circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (circuit-ir pelt export-alist circuits))
                             '()
                             pelt*)))
                   (cons "stateful_circuits"
                         (list->vector
                           (fold-right
                             (lambda (pelt circuits) (stateful-circuit-ir pelt export-alist witness-ids circuits))
                             '()
                             pelt*))))))
       ir]))

  (define-passes rust-ir-passes
    (emit-rust-ir Lnodisclose)))
