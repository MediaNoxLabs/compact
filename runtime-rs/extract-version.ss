;;; This file is part of Compact.
;;; Copyright (C) 2026 Midnight Foundation
;;; SPDX-License-Identifier: Apache-2.0
;;; Licensed under the Apache License, Version 2.0 (the "License");
;;; you may not use this file except in compliance with the License.
;;; You may obtain a copy of the License at
;;;
;;; 	http://www.apache.org/licenses/LICENSE-2.0
;;;
;;; Unless required by applicable law or agreed to in writing, software
;;; distributed under the License is distributed on an "AS IS" BASIS,
;;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;;; See the License for the specific language governing permissions and
;;; limitations under the License.

;;; Read `version = "x.y.z"` out of runtime-rs/Cargo.toml's [workspace.package]
;;; table, the way runtime/extract-version.ss reads it out of the TypeScript
;;; runtime's package.json.
;;;
;;; This exists because the two runtimes version independently. The Rust
;;; backend stamps `check_runtime_version!` into every generated crate, and
;;; that pin is checked at compile time against the `midnight-compact-runtime`
;;; crate the crate links against — so it has to come from this Cargo.toml,
;;; not from the TypeScript package.json.
;;;
;;; Deliberately not a TOML parser: it scans for the first `version` key at
;;; the start of a line (ignoring leading spaces) and takes the quoted value.
;;; In runtime-rs/Cargo.toml that is the `[package]` version of the
;;; `midnight-compact-runtime` crate itself — today the only line in the file
;;; that matches, since a dependency's inline `version = "…"` always sits
;;; after a `name = …` or inside a `{ … }` on the same line and so never
;;; starts its line.
;;;
;;; If the file is ever restructured so the first such line is not that
;;; crate's version, this stamps the wrong pin silently. The pin is
;;; const-evaluated, so the failure would at least be a hard compile error in
;;; every generated crate rather than a wrong-but-building artefact — which is
;;; how the TypeScript/Rust divergence this file exists to fix was caught.
(lambda (file)
  (let ([line
         (call-with-input-file file
           (lambda (ip)
             (let loop ()
               (let ([l (get-line ip)])
                 (cond
                   [(eof-object? l) #f]
                   [(let scan ([i 0])
                      ;; skip leading whitespace, then require `version`
                      (cond
                        [(>= i (string-length l)) #f]
                        [(char-whitespace? (string-ref l i)) (scan (fx+ i 1))]
                        [else
                         (let ([rest (substring l i (string-length l))])
                           (and (>= (string-length rest) 7)
                                (string=? (substring rest 0 7) "version")
                                rest))]))
                    => values]
                   [else (loop)])))))])
    (unless line
      (errorf 'extract-version "no `version` key found in ~a" file))
    ;; `version<spaces>=<spaces>"x.y.z"` — take what is between the quotes.
    (let* ([open (let loop ([i 0])
                   (cond
                     [(>= i (string-length line))
                      (errorf 'extract-version "no quoted value in ~s" line)]
                     [(char=? (string-ref line i) #\") i]
                     [else (loop (fx+ i 1))]))]
           [close (let loop ([i (fx+ open 1)])
                    (cond
                      [(>= i (string-length line))
                       (errorf 'extract-version "unterminated string in ~s" line)]
                      [(char=? (string-ref line i) #\") i]
                      [else (loop (fx+ i 1))]))])
      (substring line (fx+ open 1) close))))
