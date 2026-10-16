#!chezscheme

;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0

;;; Installed raw3 source-origin collector; semantic IR and admission are unchanged.
;;; Diagnostic metadata is deliberately outside the semantic JSON tree.  The
;;; weak keys do not retain speculative expression-ir results, and their values
;;; never refer back to the JSON keys.  Printing the semantic tree is unchanged.
(library (rust-source-origins)
  (export current-rust-origins make-rust-origins origin-object
          call-with-rust-origin remember-rust-origin print-rust-origins)
  (import (except (chezscheme) errorf) (utils) (json) (rust-origin-budgets))

  (define current-rust-origins (make-parameter #f))
  (define current-rust-origin (make-parameter #f))
  (define (make-rust-origins) (make-weak-eq-hashtable))
  (define (object . fields) fields)
  (define (json-object? node)
    (and (pair? node) (pair? (car node)) (string? (caar node))))
  (define (container? node) (or (json-object? node) (vector? node) (string? node)))

  ;; A context contains the actual source object, category and provenance label.
  ;; Structural JSON children inherit it; a builder promotes only its own root.
  (define (origin-object fields)
    (let ([context (current-rust-origin)] [table (current-rust-origins)])
      (let ([fields
              (if (and context (source-object? (vector-ref context 3)))
                  (map (lambda (entry)
                         (if (and (string=? (car entry) "kind") (string? (cdr entry)))
                             (cons (car entry)
                               (remember-rust-origin (string-copy (cdr entry))
                                 (vector-ref context 3) "operation" "provided" #f))
                             entry)) fields)
                  fields)])
        (when (and table context (pair? fields))
          (eq-hashtable-set! table fields
            (vector (vector-ref context 0) (vector-ref context 1) "inherited")))
        fields)))

  (define (remember-rust-origin node src category label preserve?)
    (let ([table (current-rust-origins)])
      (if (and table (source-object? src) (container? node))
          (let ([old (eq-hashtable-ref table node #f)])
            (if (and preserve? old (not (string=? (vector-ref old 2) "inherited")))
                node
                (let ([occurrence
                        ;; Distinct use sites of a reused JSON value must not
                        ;; overwrite its earlier origin.  Copy only the spine.
                        (if (and old (not (eq? src (vector-ref old 0))))
                            (cond [(string? node) (string-copy node)]
                                  [(vector? node) (vector-copy node)]
                                  [else (map values node)])
                            node)])
                  (eq-hashtable-set! table occurrence (vector src category label))
                  occurrence)))
          node)))

  (define (call-with-rust-origin category src preserve? thunk . operation-src)
    (parameterize ([current-rust-origin
                    (and (source-object? src)
                         (vector src category "provided" (and (pair? operation-src) (car operation-src))))])
      (let* ([node (thunk)]
             [kind (and (json-object? node) (assoc "kind" node))]
             [label (if (and kind (equal? (cdr kind) "coerce"))
                        "derived" "provided")])
        (if (and (pair? node) (not (json-object? node)))
            ;; Some constructor owners return an ordinary list of action JSON
            ;; objects, subsequently converted to a vector by their caller.
            (map (lambda (item) (remember-rust-origin item src category label preserve?)) node)
            (remember-rust-origin node src category label preserve?)))))

  ;; Same endpoint authority/transcoder as sourcemaps::get-source-info, without
  ;; its zero-based conversion.  Offsets are decoded Scheme-string positions,
  ;; not UTF8 bytes.  Unavailable/fake sources remain unavailable.
  (define (source-endpoint src start?)
    (guard (condition [else #f])
      (parameterize ([current-transcoder compact-input-transcoder])
        (call-with-values
          (lambda () (locate-source-object-source src start? #t))
          (case-lambda
            [() #f]
            [(path line column)
             (object (cons "line" line) (cons "column" column)
                     (cons "offset" ((if start? source-object-bfp source-object-efp) src)))])))))

  ;; These are transport containment bounds, not Compact admission limits.
  ;; Overflow produces an explicit companion error; it never truncates origins
  ;; or prevents the unchanged semantic file from being printed first.

  ;; Private metadata serializer only. The general semantic JSON printer is
  ;; unchanged. Wire nesting is shallow; semantic traversal below stays iterative.
  ;; Every control character receives a valid JSON escape, including uncommon
  ;; controls missed by the old general printer. Unicode is written as UTF8.
  (define (write-origin-json port value check-number)
    (define (string-token text)
      (put-char port #\")
      (string-for-each
        (lambda (c)
          (case c
            [(#\") (put-string port "\\\"")]
            [(#\\) (put-string port "\\\\")]
            [(#\backspace) (put-string port "\\b")]
            [(#\page) (put-string port "\\f")]
            [(#\newline) (put-string port "\\n")]
            [(#\return) (put-string port "\\r")]
            [(#\tab) (put-string port "\\t")]
            [else
             (if (< (char->integer c) 32)
                 (fprintf port "\\u~4,'0x" (char->integer c))
                 (put-char port c))])) text)
      (put-char port #\"))
    (define (emit value)
      (cond
        [(string? value) (string-token value)]
        [(vector? value)
         (put-char port #\[)
         (let loop ([index 0])
           (unless (= index (vector-length value))
             (unless (= index 0) (put-char port #\,))
             (emit (vector-ref value index))
             (loop (+ index 1))))
         (put-char port #\])]
        [(or (json-object? value) (null? value))
         (put-char port #\{)
         (let loop ([fields value] [first? #t])
           (unless (null? fields)
             (unless first? (put-char port #\,))
             (string-token (caar fields))
             (put-char port #\:)
             (emit (cdar fields))
             (loop (cdr fields) #f)))
         (put-char port #\})]
        [(boolean? value) (put-string port (if value "true" "false"))]
        [(eq? value (void)) (put-string port "null")]
        [(and (integer? value) (exact? value))
         (fprintf port "~d" (check-number value))]
        [else (internal-errorf 'rust-origin-json "unexpected metadata value")]))
    (emit value))

  (define (raw-header fields)
    (append (object (cons "schema_version" 3)
                    (cons "coordinate_system" "compact-decoded-source")
                    (cons "end_exclusive" #t)) fields))

  (define (print-rust-origins port semantic-json)
    (let* ([table (current-rust-origins)]
          [program-entry (and table (container? semantic-json)
                              (eq-hashtable-ref table semantic-json #f))]
          [program-source (and program-entry
                               (string=? (vector-ref program-entry 1) "program")
                               (source-object? (vector-ref program-entry 0))
                               (vector-ref program-entry 0))]
          [source-cache (make-eq-hashtable)]
          [file-ids (make-hashtable string-hash string=?)]
          [span-ids (make-hashtable equal-hash equal?)]
          [origin-ids (make-hashtable equal-hash equal?)]
          [files '()] [spans '()] [origins '()] [addresses '()]
          [next-file 0] [next-span 0] [next-origin 0] [next-address 0]
          [path-characters 0])
      (call/cc
        (lambda (abort)
          (define (unchecked-count value) value)
          (define (overflow resource observed limit)
            (write-origin-json port
              (raw-header
                (list (cons "error"
                        (object (cons "resource" resource)
                                (cons "observed" observed)
                                (cons "limit" limit))))) unchecked-count)
            (abort (void)))
          (define (numeric-overflow)
            ;; Bounded refusal, not a truncated/adjusted success span. The exact
            ;; out-of-width source value is deliberately not rendered here.
            (overflow "origin_numeric_width" "outside_u64" "18446744073709551615"))
          (define (u64 value)
            (unless (and (integer? value) (exact? value) (<= 0 value origin-u64-max))
              (numeric-overflow))
            value)
          (define (endpoint-value endpoint key)
            (u64 (cdr (assoc key endpoint))))
          ;; Source-object copies can denote the same Program extent.  Neither
          ;; basename, category nor source-object pointer equality establishes
          ;; this identity.  Preserve the descriptor's full path and both exact
          ;; decoded-character offsets; never adjust the virtual EOF endpoint.
          (define (program-extent? src)
            (and program-source
                 (string=?
                   (source-file-descriptor-path (source-object-sfd src))
                   (source-file-descriptor-path (source-object-sfd program-source)))
                 (= (source-object-bfp src) (source-object-bfp program-source))
                 (= (source-object-efp src) (source-object-efp program-source))))
          (define (source-extent src)
            (if (program-extent? src)
                (object (cons "kind" "compiler_context")
                        (cons "reason" "program_virtual_eof")
                        (cons "attachable" #f))
                ;; Provenance remains separate.  A physical inherited/derived
                ;; enclosure is not an independently supplied token range.
                (object (cons "kind" "physical_source"))))
          (define (write-companion companion)
            ;; Exact UTF8 preflight with a private whitespace-free serializer.
            ;; Structural bounds precede the full string + bytevector allocation.
            ;; Four MiB is a transport bound, not peak RSS or universal sufficiency.
            (let-values ([(buffer extract) (open-string-output-port)])
              (write-origin-json buffer companion u64)
              (let* ([text (extract)]
                     [bytes (bytevector-length (string->utf8 text))])
                (when (> bytes companion-byte-limit)
                  (overflow "origin_bytes" bytes companion-byte-limit))
                (put-string port text))))
          (define (source-span src)
            (let ([cached (eq-hashtable-ref source-cache src #f)])
              (or cached
                  (let* ([start (source-endpoint src #t)]
                         [end (source-endpoint src #f)]
                         [span
                           (and start end
                                (let* ([path (source-file-descriptor-path (source-object-sfd src))]
                                       [found (hashtable-ref file-ids path #f)]
                                       [id (or found next-file)])
                                  (unless found
                                    (when (>= next-file address-limit)
                                      (overflow "origin_files" (+ next-file 1) address-limit))
                                    (set! path-characters (+ path-characters (string-length path)))
                                    (when (> path-characters path-character-limit)
                                      (overflow "origin_path_characters" path-characters path-character-limit))
                                    (hashtable-set! file-ids path id)
                                    (set! next-file (+ next-file 1))
                                    (set! files (cons (object (cons "id" id) (cons "path" path)) files)))
                                  (vector id start end)))])
                    ;; #f is cached as an unavailable span too, using a sentinel.
                    (eq-hashtable-set! source-cache src (or span 'unavailable))
                    (or span 'unavailable)))))
          (define (intern-span span)
            (let ([found (hashtable-ref span-ids span #f)])
              (if found
                  found
                  (begin
                    (when (or (>= next-span address-limit) (>= next-span next-address))
                      (overflow "origin_spans" (+ next-span 1) (min address-limit next-address)))
                    (let ([id next-span])
                      (hashtable-set! span-ids span id)
                      (set! next-span (+ next-span 1))
                      (set! spans (cons span spans))
                      id)))))
          (define (origin-json origin inherited?)
            (if origin
                (let ([span (source-span (vector-ref origin 0))])
                  (if (eq? span 'unavailable)
                      (void)
                      (let* ([start (vector-ref span 1)] [end (vector-ref span 2)]
                             [tuple (vector (u64 (vector-ref span 0))
                                      (endpoint-value start "line") (endpoint-value start "column")
                                      (endpoint-value start "offset") (endpoint-value end "line")
                                      (endpoint-value end "column") (endpoint-value end "offset")
                                      (source-extent (vector-ref origin 0)))])
                        (vector (intern-span tuple)
                                (if inherited? "inherited" (vector-ref origin 2))
                                (vector-ref origin 1)))))
                (void)))
          (define (intern-origin origin)
            (if (eq? origin (void))
                (void)
                (let ([found (hashtable-ref origin-ids origin #f)])
                  (if found
                      found
                      (begin
                        ;; At most one origin is minted per visited occurrence.
                        ;; Check before allocating the next table entry/index.
                        (when (or (>= next-origin address-limit)
                                  (>= next-origin next-address))
                          (overflow "origin_records" (+ next-origin 1)
                                    (min address-limit next-address)))
                        (let ([id next-origin])
                          (hashtable-set! origin-ids origin id)
                          (set! next-origin (+ next-origin 1))
                          (set! origins (cons origin origins))
                          id))))))
          ;; A task is #(node parent edge inherited-origin).  Continuations are
          ;; #(object-tail|vector-tail children cursor parent origin).  Only one
          ;; sibling is pending at a time: broad vectors do not fill the stack.
          (let walk ([pending (list (vector semantic-json (void)
                                      (object (cons "root" #t)) #f))]
                     [pending-count 1])
            (when (> pending-count pending-limit)
              (overflow "origin_pending" pending-count pending-limit))
            (if (null? pending)
                (write-companion
                  (raw-header (list (cons "files" (list->vector (reverse files)))
                                    (cons "spans" (list->vector (reverse spans)))
                                    (cons "origins" (list->vector (reverse origins)))
                                    (cons "addresses" (list->vector (reverse addresses))))))
                (let ([task (car pending)] [rest (cdr pending)])
                  (cond
                    [(eq? (vector-ref task 0) 'object-tail)
                     (let ([fields (vector-ref task 1)])
                       (if (null? fields)
                           (walk rest (- pending-count 1))
                           (walk
                             (cons (vector (cdar fields) (vector-ref task 3)
                                           (object (cons "field" (caar fields)))
                                           (vector-ref task 4))
                                   (cons (vector 'object-tail (cdr fields) 0
                                                 (vector-ref task 3) (vector-ref task 4)) rest))
                             (+ pending-count 1))))]
                    [(eq? (vector-ref task 0) 'vector-tail)
                     (let ([children (vector-ref task 1)] [index (vector-ref task 2)])
                       (if (= index (vector-length children))
                           (walk rest (- pending-count 1))
                           (walk
                             (cons (vector (vector-ref children index) (vector-ref task 3)
                                           (object (cons "index" index)) (vector-ref task 4))
                                   (cons (vector 'vector-tail children (+ index 1)
                                                 (vector-ref task 3) (vector-ref task 4)) rest))
                             (+ pending-count 1))))]
                    [else
                     (when (= next-address address-limit)
                       (overflow "origin_addresses" (+ next-address 1) address-limit))
                     (let* ([node (vector-ref task 0)] [id next-address]
                            [own (and table (container? node) (eq-hashtable-ref table node #f))]
                            [origin (or own (vector-ref task 3))]
                            [kind (and (pair? node) (assoc "kind" node))])
                       (set! next-address (+ next-address 1))
                       (set! addresses
                         (cons (vector (vector-ref task 1) (vector-ref task 2)
                                       (if kind (cdr kind) (void))
                                       (intern-origin (origin-json origin (not own)))) addresses))
                       (cond
                         [(pair? node)
                          (walk (cons (vector 'object-tail node 0 id origin) rest) pending-count)]
                         [(vector? node)
                          (walk (cons (vector 'vector-tail node 0 id origin) rest) pending-count)]
                         [else (walk rest (- pending-count 1))]))]))))))))
)
