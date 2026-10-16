#!/usr/bin/env python3
# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Check fresh scopes.compact emission; pass the IR path, optionally a repeat path."""
import json
import sys
from pathlib import Path


def check(ir):
    stateful = {c["name"]: c for c in ir["stateful_circuits"]}
    pure = {c["name"]: c for c in ir["circuits"]}
    shadow = stateful["shadow"]
    assert [p["name"] for p in shadow["parameters"]] == ["value", "other"]
    action = shadow["actions"][0]
    local = action["bindings"][0]["name"]
    assert local not in ("value", "other")
    assert action["bindings"][0]["value"] == {"kind": "parameter", "name": "other"}
    assert action["action"]["value"]["value"] == {"kind": "parameter", "name": local}
    assert shadow["return_value"]["value"]["value"] == {"kind": "parameter", "name": "value"}
    genuine = stateful["local_return"]
    assert genuine["return_value"]["value"]["name"] == genuine["actions"][0]["bindings"][0]["name"]
    suffix = stateful["suffix"]
    formals = [p["name"] for p in suffix["parameters"]]
    assert formals == ["value", "value_Compact1", "value_Compact2", "tmp_1"]
    names = [a["bindings"][0]["name"] for a in suffix["actions"]]
    assert len(set(names + formals)) == len(names + formals)
    assert names[0] == "value_Compact3"
    assert {k: v for k, v in pure["alias_a"].items() if k != "name"} == {k: v for k, v in pure["alias_b"].items() if k != "name"}
    dollar = pure["dollars"]
    assert [p["name"] for p in dollar["parameters"]] == ["_value", "_value_Compact1"]
    left, right = dollar["body"]["bindings"][0], dollar["body"]["body"]["bindings"][0]
    assert left["name"] != right["name"]
    assert left["value"]["name"] == "_value" and right["value"]["name"] == "_value_Compact1"


if __name__ == "__main__":
    data = Path(sys.argv[1]).read_bytes()
    check(json.loads(data))
    if len(sys.argv) == 3:
        assert data == Path(sys.argv[2]).read_bytes(), "fresh repeated emission must be byte deterministic"
    print("source binding identities, public formals, aliases and deterministic emission checked")
