#! /usr/bin/env -S scheme --compile-imported-libraries --program
#!chezscheme

;;; This file is part of Compact.
;;; Copyright (C) 2025 Midnight Foundation
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

(import (except (chezscheme) errorf)
        (command-line-parsing)
        (config-params)
        (program-common)
        (passes)
        (utils))

(define (print-help)
  (print-usage #f)
  (fprintf (current-output-port) "
This program compiles the Compact source program in the file specified by
<source-pathname> and puts the resulting target files into the directory
specified by <target-directory-pathname>.  These target files include a
Typescript equivalent of the Compact source file, Zkir circuit equivalents of
the exported circuits, and proving keys created by running zkir on the zkir
circuits.

The following flags, if present, affect the compiler's behavior as follows:
  --help prints help text and exits.

  --version prints the compiler version and exits.

  --language-version prints the language version and exits.

  --ledger-version prints the ledger version and exits.  The ledger version
    is the version of the ledger that is expected by the generated code.

  --runtime-version prints the runtime version and exits.  The runtime version
    is the version of the Compact runtime JavaScript package that is used by
    generated contract code.

  --vscode causes error messages to be printed on a single line so they are
    rendered properly within the VS Code extension for Compact.

  --skip-zk causes the compiler to skip the generation of proving keys.
    Generating proving keys can be time-consuming, so this option is useful when
    debugging only the Typescript output.  The compiler also skips, after
    printing a warning message, the generation of proving keys when it cannot
    find zkir.

  --no-communications-commitment omits the contract communications commitment
    that enables data integrity for contract-to-contract calls.

  --sourceRoot <sourceRoot value> overrides the compiler's setting of the
    sourceRoot field in the generated source-map (.js.map) file.  By default,
    the compiler tries to determine a useful value based on the source and
    target-directory pathnames, but this value might not be appropriate for the
    deployed structure of the application.

  --compact-path <search list> sets the compact path, overriding the default
    value, which is the value of the environment variable COMPACT_PATH, if
    it is set, and empty otherwise.  <search list> should be a colon-separated
    (semicolon-separated under Windows) sequence of directory pathnames.
    The compact path controls where the compiler looks for include and external
    module files with non-absolute pathnames.  It always looks first relative
    to the directory of the including or importing file, then in each directory
    in the compact path from left to right.

  --trace-search causes the compiler to print a sequence of messages saying
    where it is looking for each included file and imported module source file.

  --trace-passes causes the compiler to print tracing information that is
    generally useful only to compiler developers.

  --target <language> selects a contract-code backend. It is repeatable, and the
    valid targets are ts and rust. TypeScript is emitted when --target is absent,
    so invocations that do not pass the flag are unaffected. Passing --target
    replaces that default with exactly the targets listed, so --target rust emits
    only the Rust crate (contract/lib.rs), while --target rust --target ts emits
    both. Generated Rust depends on the `midnight-compact-runtime` crate.
    ZKIR and proving keys are independent of this flag and are still generated
    unless --skip-zk is also set.
"))

(usage "<flag> ... <source-pathname> <target-directory-pathname>")

;; --target accumulation.
;;
;; `--target` is repeatable, but third_party's command-line-parsing keeps only
;; the LAST value of a repeated flag — "if a flag occurs more than once on a
;; command line, the final value of each corresponding <var> is its last
;; specified value". A plain `[(--target) (string t)]` clause would therefore
;; turn `--target rust --target ts` into just `ts`: silently wrong, which is
;; worse than unsupported. The grammar does evaluate a flag's `$ <action>` once
;; per occurrence with the var bound to the value seen, so accumulation goes
;; there.
;;
;; The action only COLLECTS; validation happens in the matched clause body.
;; Actions can fire while a clause is merely being attempted, so erroring here
;; could reject a --target value on a command line that was never going to
;; match that clause. Collection is deduped, which also makes a repeated fire
;; for one occurrence harmless.
(define known-targets '("ts" "rust"))

(define selected-targets '())

(define (collect-target! value)
  (unless (member value selected-targets)
    (set! selected-targets (append selected-targets (list value)))))

(define (known-targets-string)
  (let loop ([ts known-targets] [acc ""])
    (cond
      [(null? ts) acc]
      [(string=? acc "") (loop (cdr ts) (car ts))]
      [else (loop (cdr ts) (string-append acc ", " (car ts)))])))

;; Reject unknown targets, naming the valid ones. Runs only after a clause has
;; matched, so the message always describes a command line we really parsed.
(define (check-targets!)
  (for-each
    (lambda (t)
      (unless (member t known-targets)
        (fprintf (current-error-port)
                 "compactc: unknown --target ~s; valid targets are ~a\n"
                 t (known-targets-string))
        (exit 1)))
    selected-targets))

;; `--rust` / `--skip-ts` remain as undocumented aliases so existing callers
;; (this fork's harness, and MediaNoxLabs/midnight-identity, which runs
;; `compactc --rust --skip-ts`) keep working. Mixing the two spellings is an
;; error rather than a silent precedence rule, so no invocation can be read two
;; ways. The aliases are fork-transitional: upstream never shipped them, so
;; there is nothing there to deprecate and they must not be carried upstream.
(define (check-no-alias-mixing! rust? skip-ts?)
  (when (and (pair? selected-targets) (or rust? skip-ts?))
    (fprintf (current-error-port)
             "compactc: --target cannot be combined with --rust or --skip-ts; use --target alone\n")
    (exit 1)))

;; Absent --target, fall back to the aliases; otherwise the listed targets are
;; authoritative — `rust` present means emit Rust, `ts` absent means skip it.
(define (resolve-emit-rust rust?)
  (if (pair? selected-targets) (and (member "rust" selected-targets) #t) rust?))

(define (resolve-skip-ts skip-ts?)
  (if (pair? selected-targets) (not (member "ts" selected-targets)) skip-ts?))

(parameterize ([reset-handler abort])
  (command-line-case (command-line)
    [((flags [(--help) $ (begin (print-help) (exit))]
             [(--version) $ (begin (print-compiler-version) (exit))]
             [(--language-version) $ (begin (print-language-version) (exit))]
             [(--ledger-version) $ (begin (print-ledger-version ?--feature-zkir-v3) (exit))]
             [(--runtime-version) $ (begin (print-runtime-version) (exit))]
             [(--vscode)]
             [(--skip-zk)]
             [(--no-communications-commitment)]
             [(--sourceRoot) (string source-root)]
             [(--compact-path) (string search-list)]
             [(--trace-search)]
             [(--trace-passes)]
             [(--feature-zkir-v3)]
             [(--target) (string target-language) $ (collect-target! target-language)]
             [(--rust)]
             [(--skip-ts)])
      (string source-pathname)
      (string target-directory-pathname))
     (check-pathname source-pathname)
     (check-pathname target-directory-pathname)
     (check-targets!)
     (check-no-alias-mixing! ?--rust ?--skip-ts)
     (parameterize ([trace-passes ?--trace-passes]
                    [skip-zk ?--skip-zk]
                    [no-communications-commitment ?--no-communications-commitment]
                    [feature-zkir-v3 ?--feature-zkir-v3]
                    [emit-rust (resolve-emit-rust ?--rust)]
                    [skip-ts (resolve-skip-ts ?--skip-ts)]
                    [compact-path (if ?--compact-path (split-search-path search-list) (compact-path))]
                    [trace-search ?--trace-search])
       (when source-root (register-source-root! source-root))
       (handle-exceptions ?--vscode
         (generate-everything source-pathname target-directory-pathname)))]
    [((flags [(--help) $ (begin (print-help) (exit))]
             [(--version) $ (begin (print-compiler-version) (exit))]
             [(--language-version) $ (begin (print-language-version) (exit))]
             [(--ledger-version) $ (begin (print-ledger-version ?--feature-zkir-v3) (exit))]
             [(--runtime-version) $ (begin (print-runtime-version) (exit))]
             [(--feature-zkir-v3)]
             [(--target) (string target-language) $ (collect-target! target-language)]
             [(--rust)]
             [(--skip-ts)])
      (string arg) ...)
     (print-usage #t)
     (exit 1)]))
