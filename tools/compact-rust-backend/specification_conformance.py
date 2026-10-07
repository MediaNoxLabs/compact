#!/usr/bin/env python3

# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#  	http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Inventory actual Nanopass grammar; check mapping/drift, never prove equivalence.

Extract with --extract PATH --scheme PATH --nanopass-root PATH. Check an
existing extraction with --inventory PATH --baseline PATH. With neither flag,
check the source-bound embedded baseline (no fresh extraction). No mode builds
compiler artifacts or rewrites the baseline. Unknown semantics remain unknown.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
GRAMMARS = ("Lsrc", "Lnodisclose")
SOURCES = tuple("compiler/" + name + ".ss" for name in (
    "langs", "field", "nanopass-extension", "compiler-version", "language-version", "version"
))


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(value):
    return hashlib.sha256(canonical(value).encode()).hexdigest()


def file_hash(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_json(text):
    def unique_pairs(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(text, object_pairs_hook=unique_pairs)


def normalize_grammar(raw):
    """Normalize declaration order only; preserve each production's argument order."""
    if not isinstance(raw, list) or len(raw) < 4 or raw[:1] != ["define-language"]:
        raise ValueError("expected actual Nanopass define-language export")
    name = raw[1]
    if not isinstance(name, str):
        raise ValueError("invalid language name")
    requirements = []
    seen = set()
    owners = set()
    entries = []

    def add(kind, owner, production):
        identifier = f"{name}/{owner}/{kind}/{digest(production)[:16]}"
        if identifier in seen:
            raise ValueError(f"duplicate grammar requirement: {identifier}")
        seen.add(identifier)
        requirements.append(dict(id=identifier, kind=kind, nonterminal=owner, production=production))

    for clause in raw[2:]:
        if not isinstance(clause, list) or len(clause) < 2 or not isinstance(clause[0], str):
            raise ValueError("malformed grammar clause")
        owner = clause[0]
        if owner in owners:
            raise ValueError(f"duplicate grammar owner: {owner}")
        owners.add(owner)
        if owner == "entry":
            if len(clause) != 2 or not isinstance(clause[1], str):
                raise ValueError("malformed entry declaration")
            entries.append(clause[1])
            add("entry", owner, clause[1])
        elif owner == "terminals":
            for terminal in clause[1:]:
                if not isinstance(terminal, list) or len(terminal) != 2 or not isinstance(terminal[1], list):
                    raise ValueError("malformed terminal declaration")
                add("terminal", owner, [terminal[0], sorted(terminal[1])])
        else:
            if not isinstance(clause[1], list) or not all(isinstance(x, str) for x in clause[1]):
                raise ValueError("malformed nonterminal declaration")
            add("nonterminal", owner, sorted(clause[1]))
            for production in clause[2:]:
                add("production", owner, production)
    if sum(x["kind"] == "entry" for x in requirements) != 1:
        raise ValueError("grammar must have exactly one entry")
    if entries[0] not in owners - {"entry", "terminals"}:
        raise ValueError("entry references missing nonterminal")
    requirements.sort(key=lambda x: x["id"])
    return {"sha256": digest(requirements), "requirements": requirements}


# JSON is emitted by Scheme itself; no regex reconstruction of current grammar.
SCHEME_EXPORT = r'''
(import (chezscheme))
(library-directories LIBRARIES)
(library-extensions '((".chezscheme.sls" . ".conformance-no-object") (".ss" . ".conformance-no-object") (".sls" . ".conformance-no-object")))
(compile-imported-libraries #f)
(import (nanopass) (langs) (compiler-version) (language-version))
(define (json-string value)
  (display #\")
  (string-for-each
    (lambda (c)
      (case c
        [(#\") (display "\\\"")]
        [(#\\) (display "\\\\")]
        [else
         (if (< (char->integer c) 32)
             (begin (display "\\u")
               (let ([s (number->string (char->integer c) 16)])
                 (display (make-string (- 4 (string-length s)) #\0)) (display s)))
             (display c))])) value)
  (display #\"))
(define (json-value value)
  (cond
    [(list? value)
     (display "[")
     (let loop ([xs value] [first? #t])
       (unless (null? xs)
         (unless first? (display ","))
         (json-value (car xs)) (loop (cdr xs) #f)))
     (display "]")]
    [(symbol? value) (json-string (symbol->string value))]
    [(string? value) (json-string value)]
    [(boolean? value) (display (if value "true" "false"))]
    [(and (integer? value) (exact? value)) (display value)]
    [else (error 'grammar-export "unsupported grammar datum" value)]))
(json-value (list compiler-version-string language-version-string
                  (language->s-expression Lsrc) (language->s-expression Lnodisclose)
                  (scheme-version)))
(newline)
'''


def extract(repo, scheme, nanopass, timeout=60):
    """Load current source libraries with object loading disabled, in scratch."""
    repo, nanopass = Path(repo).resolve(), Path(nanopass).resolve()
    scheme = Path(scheme).resolve()
    if not scheme.is_file() or not os.access(scheme, os.X_OK):
        raise ValueError(f"Chez executable unavailable: {scheme}")
    if not (nanopass / "nanopass.ss").is_file():
        raise ValueError(f"Nanopass source unavailable: {nanopass}")
    before = {path: file_hash(repo / path) for path in SOURCES}
    libraries = "'(\"" + str(repo / "compiler").replace("\\", "\\\\").replace('"', '\\"') + '\" \"' + str(nanopass).replace("\\", "\\\\").replace('"', '\\"') + '\")'
    script = SCHEME_EXPORT.replace("LIBRARIES", libraries)
    with tempfile.TemporaryDirectory(prefix="compact-grammar-") as scratch:
        program = Path(scratch) / "export.ss"
        program.write_text(script)
        env = dict(os.environ)
        env.pop("CHEZSCHEMELIBDIRS", None)
        env.pop("CHEZSCHEMELIBEXTS", None)
        result = subprocess.run([str(scheme), "--script", str(program)], cwd=scratch,
                                env=env, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError(f"live Nanopass extraction failed ({result.returncode}): {result.stderr.strip()}")
    try:
        raw = load_json(result.stdout)
    except json.JSONDecodeError as error:
        raise ValueError("live extractor returned invalid JSON; no fallback grammar") from error
    if not isinstance(raw, list) or len(raw) != 5 or not all(isinstance(x, str) for x in raw[:2]):
        raise ValueError("invalid live grammar envelope")
    grammars = {x[1]: normalize_grammar(x) for x in raw[2:4]}
    if set(grammars) != set(GRAMMARS):
        raise ValueError("live extraction must contain Lsrc and Lnodisclose")
    after = {path: file_hash(repo / path) for path in SOURCES}
    if before != after:
        raise ValueError("compiler sources changed during live extraction")
    return {"format_version": 1, "compiler_version": raw[0], "language_version": raw[1],
            "grammars": grammars, "sources": before,
            "extraction": {"mechanism": "Nanopass language->s-expression; source-only Chez import",
                           "scheme": str(scheme), "scheme_version": raw[4], "scheme_sha256": file_hash(scheme),
                           "nanopass_sources": {str(p.relative_to(nanopass)): file_hash(p)
                                                for p in sorted(nanopass.rglob("*"))
                                                if p.is_file() and p.suffix in (".ss", ".sls")},
                           "script_sha256": hashlib.sha256(script.encode()).hexdigest(),
                           "stderr": result.stderr.strip()}}


def inventory_ids(inventory):
    if inventory.get("format_version") != 1 or set(inventory.get("grammars", {})) != set(GRAMMARS):
        raise ValueError("unsupported inventory format or missing language")
    ids = set()
    for language, grammar in inventory["grammars"].items():
        requirements = grammar["requirements"]
        if digest(requirements) != grammar["sha256"]:
            raise ValueError(f"invalid normalized grammar digest: {language}")
        for row in requirements:
            expected = f"{language}/{row['nonterminal']}/{row['kind']}/{digest(row['production'])[:16]}"
            if row["id"] != expected or expected in ids:
                raise ValueError("invalid or duplicate requirement identity")
            ids.add(expected)
    return ids


def checked_path(repo, name):
    path = (Path(repo) / name).resolve()
    if not path.is_relative_to(Path(repo).resolve()):
        raise ValueError(f"source path escapes repository: {name}")
    return path


def formal_drift(repo, artifacts, inventory):
    """Compare historical JSON grammar and Agda version stamps, not Agda semantics."""
    rows = []
    for artifact in artifacts:
        path = checked_path(repo, artifact["path"])
        if file_hash(path) != artifact["sha256"]:
            raise ValueError(f"formal artifact changed: {artifact['path']}")
        row = dict(artifact)
        row["qualification"] = "No Agda typecheck, constructor proof or operational equivalence claim"
        if path.suffix == ".json":
            raw = load_json(path.read_text())
            if not isinstance(raw, list) or len(raw) < 5:
                raise ValueError("malformed historical grammar JSON")
            versions = dict(raw[:2])
            historical = normalize_grammar(raw[2:])
            language = raw[3]
            current = inventory["grammars"].get(language)
            if current is None:
                row["comparison"] = "outside selected grammar inventory"
            else:
                old_ids = {x["id"] for x in historical["requirements"]}
                new_ids = {x["id"] for x in current["requirements"]}
                row.update(added_requirements=sorted(new_ids - old_ids),
                           removed_requirements=sorted(old_ids - new_ids),
                           same_grammar=historical["sha256"] == current["sha256"])
            row["versions"] = versions
        elif path.suffix == ".agda":
            # Only inspect historical header stamps. Never parse current grammar from text.
            row["versions"] = dict(re.findall(r"^-- \*+ (Compiler version|Language version): (\S+)$", path.read_text(), re.M))
        else:
            raise ValueError(f"unsupported formal artifact: {artifact['path']}")
        row["same_versions"] = row["versions"] == {
            "Compiler version": inventory["compiler_version"],
            "Language version": inventory["language_version"],
        }
        stale = not row["same_versions"] or row.get("same_grammar") is False
        if stale and artifact.get("status") != "known-stale":
            raise ValueError(f"unacknowledged formal drift: {artifact['path']}")
        row["current_disposition"] = "known formal debt" if stale else "syntax/version match only"
        rows.append(row)
    return rows




SUPPORT_VALUES = {
    "frontend": {"unknown", "source-reviewed", "frontend-handled"},
    "private_ir": {"unknown", "derived-conditional", "lowered-before-backend", "represented-subset"},
    "native": {"unknown", "implemented-subset", "selected-cases-observed", "source-reviewed-only", "not-applicable"},
    "recording": {"unknown", "profile-gated", "profile-gated-unqualified-for-this-requirement"},
    "observed_call": {"unknown", "profile-gated", "profile-gated-unqualified-for-this-requirement"},
    "proof_applicability": {"unknown"},
}


def reference_headings(text):
    """Reference navigation accounting, not a parser for normative sentences."""
    headings, fence = [], None
    lines = text.splitlines()
    for number, line in enumerate(lines, 1):
        marker = re.match(r"^\s*(`{3,}|~{3,})", line)
        if marker:
            token = marker[1]
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
            continue
        if fence is None:
            match = re.match(r"^(#{1,6}) (.+)$", line)
            if match:
                headings.append({"level": len(match[1]), "title": match[2], "line": number})
    for index, heading in enumerate(headings):
        following = next((x for x in headings[index + 1:] if x["level"] <= heading["level"]), None)
        heading["end_line"] = following["line"] - 1 if following else len(lines)
    return headings


def validate_semantic_inventory(repo, baseline, errors):
    for collection in ("evidence", "requirements", "reference_headings"):
        if collection not in baseline or not isinstance(baseline[collection], list):
            raise ValueError(f"missing or invalid semantic collection: {collection}")
    evidence = baseline["evidence"]
    requirements = baseline["requirements"]
    for label, rows in (("evidence", evidence), ("requirements", requirements)):
        ids = [row["id"] for row in rows]
        if len(set(ids)) != len(ids):
            errors.append({"kind": "duplicate-semantic-id", "collection": label})
    evidence_ids = {row["id"] for row in evidence}
    for row in baseline["semantic_families"] + requirements:
        if any(item not in evidence_ids for item in row.get("evidence_ids", [])):
            errors.append({"kind": "dangling-evidence-reference", "id": row["id"]})
        if "support" in row:
            support = row["support"]
            if set(support) - set(SUPPORT_VALUES) - {"executed_proof"}:
                errors.append({"kind": "unknown-support-axis", "id": row["id"]})
            for key, allowed in SUPPORT_VALUES.items():
                if support.get(key) not in allowed:
                    errors.append({"kind": "invalid-support-status", "id": row["id"], "axis": key})
            if support.get("executed_proof", False) is not False:
                errors.append({"kind": "unsupported-proof-claim", "id": row["id"]})
            if support.get("native") == "selected-cases-observed" and not row.get("evidence_ids"):
                errors.append({"kind": "observed-status-without-evidence", "id": row["id"]})
    if "reference_headings" in baseline:
        headings = baseline["reference_headings"]
        if not headings or len({x["source"]["path"] for x in headings}) != 1:
            errors.append({"kind": "invalid-reference-heading-inventory"})
        else:
            source = checked_path(repo, headings[0]["source"]["path"])
            actual = reference_headings(source.read_text())
            recorded = [{k: x[k] for k in ("level", "title", "line", "end_line")} for x in headings]
            if actual != recorded:
                errors.append({"kind": "reference-heading-drift"})


def validate_references(repo, value, family_ids, errors, location="baseline"):
    """Check repository anchors without relabeling historical artifact receipts."""
    if isinstance(value, list):
        for index, item in enumerate(value):
            validate_references(repo, item, family_ids, errors, f"{location}[{index}]")
    elif isinstance(value, dict):
        if "path" in value:
            path = checked_path(repo, value["path"])
            if value.get("sha256") != file_hash(path):
                errors.append({"kind": "anchor-source-drift", "location": location, "path": value["path"]})
            if "line" in value and (type(value["line"]) is not int or
                                   not 1 <= value["line"] <= len(path.read_text().splitlines())):
                errors.append({"kind": "invalid-anchor-line", "location": location})
        if "family" in value and value["family"] not in family_ids:
            errors.append({"kind": "dangling-family-reference", "location": location,
                           "family": value["family"]})
        for key, item in value.items():
            validate_references(repo, item, family_ids, errors, f"{location}.{key}")


def check(repo, inventory, baseline):
    """Require exact grammar classification and reviewed source identities."""
    if baseline.get("format_version") != 1:
        raise ValueError("unsupported conformance baseline format")
    actual_ids = inventory_ids(inventory)
    expected = baseline["grammar_inventory"]
    expected_ids = inventory_ids(expected)
    errors = []
    if actual_ids != expected_ids:
        errors.append({"kind": "grammar-requirements-drift", "added": sorted(actual_ids - expected_ids),
                       "removed": sorted(expected_ids - actual_ids)})
    for key in ("compiler_version", "language_version", "sources"):
        if inventory.get(key) != expected.get(key):
            errors.append({"kind": "inventory-drift", "field": key})
    sources = dict(baseline.get("sources", {}))
    for name, sha in inventory["sources"].items():
        if name in sources and sources[name] != sha:
            errors.append({"kind": "conflicting-source-identity", "path": name})
        sources[name] = sha
    for key in ("compiler_version", "language_version"):
        if key in baseline.get("baseline", {}) and baseline["baseline"][key] != inventory[key]:
            errors.append({"kind": "baseline-version-drift", "field": key})
    for name, sha in sources.items():
        if file_hash(checked_path(repo, name)) != sha:
            errors.append({"kind": "source-drift", "path": name})
    compatibility_keys = {"private_ir_version": "ir_schema", "runtime_abi": "runtime_abi",
                          "native_ledger_version": "ledger_version"}
    if any(key in baseline.get("baseline", {}) for key in compatibility_keys):
        path = "tools/compact-rust-backend/src/compatibility.json"
        if path not in sources:
            errors.append({"kind": "unbound-compatibility-metadata", "path": path})
        compatibility = load_json(checked_path(repo, path).read_text())
        for key, source_key in compatibility_keys.items():
            expected_value = compatibility[source_key]
            if key == "native_ledger_version":
                expected_value = expected_value.removeprefix("ledger-")
            if baseline["baseline"].get(key) != expected_value:
                errors.append({"kind": "compatibility-version-drift", "field": key})
    families = baseline["semantic_families"]
    family_ids = [x["id"] for x in families]
    if len(set(family_ids)) != len(family_ids):
        raise ValueError("duplicate semantic family")
    mapping = baseline["grammar_classifications"]
    if set(mapping) != actual_ids:
        errors.append({"kind": "classification-drift", "missing": sorted(actual_ids - set(mapping)),
                       "orphaned": sorted(set(mapping) - actual_ids)})
    for identifier, classification in mapping.items():
        if classification.get("family") not in family_ids:
            errors.append({"kind": "unknown-family", "id": identifier, "family": classification.get("family")})
    validate_semantic_inventory(repo, baseline, errors)
    validate_references(repo, baseline, family_ids, errors)
    formal = formal_drift(repo, baseline.get("formal_artifacts", []), inventory)
    if not formal:
        errors.append({"kind": "missing-formal-artifact-inventory"})
    return {"format_version": 1, "status": "passed" if not errors else "failed",
            "requirements": len(actual_ids), "families": len(families), "errors": errors,
            "formal_artifacts": formal,
            "evidence_mode": "source-bound extraction receipt; only --extract invokes live Scheme",
            "qualification": "Grammar/source/classification drift only. Unknown support stays unknown; no semantic equivalence or formal proof established."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument("--extract", type=Path)
    parser.add_argument("--scheme", type=Path)
    parser.add_argument("--nanopass-root", type=Path)
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--baseline", type=Path, default=Path(__file__).with_suffix(".json"))
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    try:
        if args.extract and args.inventory:
            raise ValueError("--extract and --inventory are mutually exclusive")
        if args.extract:
            if not args.scheme or not args.nanopass_root:
                raise ValueError("--extract requires --scheme and --nanopass-root")
            inventory = extract(args.repo_root, args.scheme, args.nanopass_root)
            args.extract.write_text(json.dumps(inventory, indent=2) + "\n")
        else:
            baseline = load_json(args.baseline.read_text())
            inventory = load_json(args.inventory.read_text()) if args.inventory else baseline["grammar_inventory"]
            result = check(args.repo_root, inventory, baseline)
            if not args.inventory:
                result["evidence_mode"] = "cached source-bound baseline check; no fresh extraction"
            output = json.dumps(result, indent=2) + "\n"
            if args.report:
                args.report.write_text(output)
            else:
                print(output, end="")
            return 0 if result["status"] == "passed" else 1
    except (ValueError, KeyError, TypeError, OSError, subprocess.TimeoutExpired) as error:
        parser.exit(1, f"specification conformance: {error}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
