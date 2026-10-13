# This file is part of Compact.
# Copyright (C) 2026 Midnight Foundation
# SPDX-License-Identifier: Apache-2.0
"""Additive native Let symbol repair; generic control/body fold remains pending."""
import copy
import json
import re
from recording_admission_inventory_jubjub import canonical, digest
from recording_admission_inventory_aligned_test import _TextSnapshot
from recording_admission_inventory_shared_coercion import function

MANIFEST_SHA = '7b9a7ff08a3fbd39b43049017fec72e7149ec6327adbfab90aa55679a8084130'
BASELINE_COMMIT = '4fea3b422242505f739e2431a402aea8ec033eb3'
BASE = 'tools/compact-rust-backend/src'
LIB = BASE + '/lib.rs'
NATIVE = BASE + '/pure_native.rs'
TEST_MODULE = BASE + '/pure_native/tests.rs'
LOCAL = BASE + '/pure_native/local.rs'
SELECTOR = BASE + '/recorded/profile_selection.rs'
OWNERS = {LIB, NATIVE, TEST_MODULE}


def authenticate(extension):
    if (canonical(extension) != MANIFEST_SHA or extension.get('schema_version') != 1
            or extension.get('issue') != 629 or extension.get('parent_issue') != 622 or extension.get('slice') != 'D4a'
            or extension.get('baseline_commit') != BASELINE_COMMIT
            or extension.get('new_retired_route_ids') != []
            or set(extension.get('owners', {})) != OWNERS):
        raise ValueError('symbols629D4a: unreviewed extension or incomplete owner set')


def inverse_regions(source, row, owner):
    """Restore only exact approved regions; mutations outside them survive."""
    for edit in row['predecessor_reconstruction']:
        if not edit['current'] or source.count(edit['current']) != 1:
            raise ValueError('symbols629D4a: current inverse region changed: ' + owner)
        source = source.replace(edit['current'], edit['previous'], 1)
    return source


def predecessor_sources(root, extension):
    authenticate(extension)
    sources = {}
    for owner, row in extension['owners'].items():
        source = (root / owner).read_bytes().decode()
        if digest(source.encode()) != row['current_sha256']:
            raise ValueError('symbols629D4a: current owner changed: ' + owner)
        source = inverse_regions(source, row, owner)
        if digest(source.encode()) != row['previous_sha256']:
            raise ValueError('symbols629D4a: signed D3 predecessor differs: ' + owner)
        sources[owner] = source
    for label in ('new_and_shared_sources', 'historical_manifests', 'oracle_authorities'):
        for owner, expected in extension[label].items():
            if digest((root / owner).read_bytes()) != expected:
                raise ValueError('symbols629D4a: ' + label + ' changed: ' + owner)
    if len(extension['historical_manifests']) != 76:
        raise ValueError('symbols629D4a: all 76 scoped historical manifests required')
    return sources


class _Checkpoint:
    def __init__(self, root, extension):
        self._root, self._sources = root, predecessor_sources(root, extension)
    def __truediv__(self, owner):
        return _TextSnapshot(self._sources[owner]) if owner in self._sources else self._root / owner


def checkpoint(root, extension):
    return _Checkpoint(root, extension)


def latest_checkpoint(root, baseline_path):
    path = baseline_path.with_name('recording_admission_inventory.lexical-symbols-629d4a.json')
    return checkpoint(root, json.loads(path.read_text())) if path.exists() else root


def require(source, anchor, count=1):
    if source.count(anchor) != count:
        raise ValueError('symbols629D4a: native Let symbol boundary changed: ' + anchor)


def ordered(source, anchors):
    for anchor in anchors:
        require(source, anchor)
    positions = [source.index(x) for x in anchors]
    if positions != sorted(positions):
        raise ValueError('symbols629D4a: first-error ordering changed')


def rust_tokens(source):
    """Relocation comparison only: preserve quoted literal contents exactly."""
    return re.findall(r'"(?:\\.|[^"\\])*"|\w+|[^\s]', source)


def unique_variants(source):
    # Native syn::Expr constructors in the old family are output, not source
    # variants. Root adapters and the signed old source map use bare Expr.
    return set(re.findall(r'(?<!:)\bExpr::(\w+)', source))


def match_body(source, marker):
    parts = rust_tokens(source)
    start = rust_tokens(marker)
    found = [i for i in range(len(parts) - len(start) + 1) if parts[i:i + len(start)] == start]
    if len(found) != 1:
        raise ValueError('symbols629D4a: checked native match boundary changed')
    opening = found[0] + len(start)
    if parts[opening] != '{':
        raise ValueError('symbols629D4a: checked native match opening changed')
    depth = 0
    for index in range(opening, len(parts)):
        if parts[index] == '{':
            depth += 1
        elif parts[index] == '}':
            depth -= 1
            if depth == 0:
                return parts[opening:index + 1]
    raise ValueError('symbols629D4a: incomplete native match')


def source_closure(root, extension, previous):
    text = lambda owner: (root / owner).read_bytes().decode()
    lib = text(LIB)
    preserved = {}
    for name in ('pure_aggregate_expression', 'pure_crypto_expression', 'pure_arithmetic_expression',
                 'pure_conversion_expression', 'pure_leaf_expression', 'native_fragment_with_calls', 'expression_with_calls'):
        if function(lib, name) != function(previous[LIB], name):
            raise ValueError('symbols629D4a: accepted semantic owner/entry changed: ' + name)
        preserved[name] = digest(function(lib, name).encode())
    current_control = function(lib, 'pure_control_fragment')
    for label, row in extension['let_regions'].items():
        if current_control.count(row['current']) != 1:
            raise ValueError('symbols629D4a: current Let ' + label + ' boundary changed')
    reconstructed = inverse_regions(current_control, extension['owners'][LIB], LIB)
    if reconstructed != function(previous[LIB], 'pure_control_fragment'):
        raise ValueError('symbols629D4a: control owner outside approved Let slots changed')
    branch = current_control[current_control.index('        Expr::Let { bindings, body } => {'):]
    ordered(branch, ('let checked_name = pure_semantics::identifier::check_name(&binding.name)?;',
                     'let (value, actual) = expression_with_calls(&binding.value, &locals, circuits)?;',
                     'if actual != binding.ty', 'let local_ty = rust_type(&binding.ty)?;',
                     'let local_name = pure_native::local::allocate(',
                     'statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));',
                     'locals.insert(binding.name.as_str(), (&binding.ty, local_name));',
                     'native_fragment_with_calls(body, &locals, circuits)?'))
    require(branch, 'locals.iter().map(|(key, (_, symbol))| (*key, symbol))')
    if 'syn::Ident::new(' in branch or 'ident(&binding.name)?' in branch:
        raise ValueError('symbols629D4a: unchecked or duplicate Let name validation returned')
    local = text(LOCAL)
    for anchor in ('checked_name: &CheckedName', 'raw_source_key: &str',
                   "occupied: impl IntoIterator<Item = (&'a str, &'a syn::Ident)>",
                   '.filter(|(key, _)| *key != raw_source_key)',
                   'spelling.strip_prefix("r#").unwrap_or(&spelling).to_owned()',
                   'let base = format!("__compact_local_{}", checked_name.text());'):
        require(local, anchor)
    ordered(local, ('let mut candidate = base.clone();', 'let mut suffix = 1_usize;',
                    'while occupied.contains(&candidate)', 'candidate = format!("{base}_{suffix}");',
                    'suffix += 1;', 'syn::Ident::new(&candidate, proc_macro2::Span::call_site())'))
    for forbidden in ('RenderError', 'ir::Expr', 'ir::Type', '&Type', 'check_name(', 'ident(',
                      'rust_type(', 'plan_coercion(', 'expression_with_calls(', 'native_fragment_with_calls(',
                      'parse_quote!', 'static mut', 'random('):
        if forbidden in local:
            raise ValueError('symbols629D4a: output symbol allocator acquired source checking/global state')
    require(local, '#[cfg(test)]\nmod tests;')
    native = text(NATIVE)
    if native.replace('\npub(crate) mod local;\n', '', 1) != previous[NATIVE]:
        raise ValueError('symbols629D4a: existing native module owner changed')
    tests = text(TEST_MODULE)
    require(tests, 'mod local_symbols;')
    if tests.replace('mod local_symbols;\n', '', 1) != previous[TEST_MODULE]:
        raise ValueError('symbols629D4a: existing native test owner changed')
    for owner, expected in extension['new_and_shared_sources'].items():
        if digest((root / owner).read_bytes()) != expected:
            raise ValueError('symbols629D4a: protected current source changed: ' + owner)
    return {'preserved_functions': preserved,
            'native_let': 'same name-before-initializer and type/carrier order; capture checked spelling once; raw source lookup keys unchanged',
            'allocation': 'lowest free prefixed checked word; live occupied native spellings normalize r#; only exact same raw key exempt for shadowing',
            'output_qualification': 'ordinary free spelling stable; corrected invalid/colliding accepted-name cases deliberately alpha-renamed; no blanket old/current parser-panic equivalence',
            'scope': 'native Let only; Map and parameter allocators, generic control/body fold remain pending',
            'body_syntax_bridge': 'still constructs syntax; generic control/body-binding/full CheckSink pending622',
            'resource_policy': 'existing DEFAULT/name/type/carrier/fragment policies unchanged; no cumulative memory guarantee',
            'new_retired_routes': [], 'claim': 'native Let symbol repair; source inventory is not language coverage'}


def verify_extension(root, baseline, extension, historical_verify, selector_signature):
    report = {'schema_version': 1, 'claim': 'source inventory; not semantic coverage',
              'route_count': len(baseline.get('routes', [])), 'retired_routes': [],
              'route_checks': [], 'source_changes': [], 'errors': [], 'passed': False}
    try:
        checked = checkpoint(root, extension)
        prior = historical_verify(checked)
        if not prior['passed']:
            report['errors'] = ['symbols629D4a: predecessor checks failed', *prior['errors']]
            return report
        expected = {'reviewed_shared_aggregate_extension': 'D3',
                    'reviewed_shared_crypto_extension': 'D2',
                    'reviewed_shared_arithmetic_extension': 'D1c',
                    'reviewed_shared_conversion_extension': 'D1b',
                    'reviewed_shared_leaf_extension': 'D1a', 'reviewed_unit_fragment_extension': 'C1+C2'}
        if (any(prior.get(key, {}).get('issue') != 622 or prior.get(key, {}).get('slice') != stage
                for key, stage in expected.items())
                or prior.get('reviewed_shared_coercion_extension', {}).get('issue') != 627):
            raise ValueError('symbols629D4a: mandatory actual D3/D2/D1c/D1b/D1a/C/B predecessor missing')
        report = copy.deepcopy(prior)
        report['passed'] = False
        if prior['route_count'] != 84 or len(prior['retired_routes']) != 69:
            raise ValueError('symbols629D4a: existing denominator/retirements changed')
        selector = selector_signature((root / SELECTOR).read_bytes().decode())
        if selector != extension['current_selector'] or selector != prior['current_selector']:
            raise ValueError('symbols629D4a: recording authority changed')
        closure = source_closure(root, extension, checked._sources)
        if closure != extension['production_closure']:
            raise ValueError('symbols629D4a: reviewed lexical symbol closure changed')
        report['reviewed_lexical_symbols_extension'] = {
            'issue': 629, 'parent_issue': 622, 'slice': 'D4a', 'claim': extension['claim'], 'new_retired_routes': [],
            'predecessor_commit': BASELINE_COMMIT, 'historical_manifest_count': 76,
            'original_oracles': 49, 'production_closure': closure,
            'owners': copy.deepcopy(extension['owners'])}
        report['passed'] = True
    except (OSError, ValueError, KeyError, UnicodeError, StopIteration, IndexError) as error:
        report['errors'].append(str(error))
    return report
