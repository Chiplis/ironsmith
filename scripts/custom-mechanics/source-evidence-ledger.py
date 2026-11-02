#!/usr/bin/env python3
"""Source availability and attributable native evidence are independent facts.

This program only reads source/fixture/test/log files. It never invokes tests,
accepts row support_status as proof, or attributes a target-wide pass to cards
without an explicit source/test binding and immutable run inputs.
"""
import argparse
import collections
import functools
import hashlib
import json
import re
import tomllib
from pathlib import Path
from native_batch_evidence import read_native_batch


@functools.lru_cache(maxsize=None)
def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def json_digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()


def scope(value):
    return 'revolution' if value in ('rev', 'revolution') else value


def physical_key(value, namespace):
    if not isinstance(value, str) or not value.strip():
        return None
    value = value.strip()
    if ':' in value:
        head, tail = value.split(':', 1)
        if head in ('rev', 'revolution', 'msem'):
            return f'{scope(head)}:{tail}' if scope(head) == namespace else None
    return f'{namespace}:{value}'


def read_json(path):
    return json.loads(Path(path).read_text())


def code_digest(root):
    files = sorted(path for path in (root / 'crates').rglob('*')
                   if path.is_file() and path.suffix in ('.rs', '.toml', '.tsv') and 'target' not in path.parts)
    for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'):
        if (root / name).is_file():
            files.append(root / name)
    return json_digest({str(path.relative_to(root)): digest(path) for path in files})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--inventory', required=True, action='append', type=Path,
                        help='Explicit source selection coverage or discovery ledger; never selected by display-name guessing')
    parser.add_argument('--fixtures', action='append', default=[], type=Path)
    parser.add_argument('--bindings', action='append', default=[], type=Path)
    parser.add_argument('--runs', action='append', default=[], type=Path)
    parser.add_argument('--native-batch', action='append', default=[], type=Path, help='Actual parent batch.json with pre-build fingerprint packet and sibling results.jsonl')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    root = args.root.resolve()
    resolve = lambda value: Path(value) if Path(value).is_absolute() else root / value
    display = lambda path: str(path.relative_to(root)) if path.is_relative_to(root) else str(path)
    seeds = {}
    for file in ('revolution.json', 'msem-source.json'):
        for row in read_json(root / 'fixtures/custom_mechanics' / file)['mechanics']:
            if row['id'] in seeds:
                raise ValueError(f'duplicate immutable seed: {row["id"]}')
            seeds[row['id']] = row
    if len(seeds) != 971:
        raise ValueError(f'immutable source inventory has {len(seeds)} rows, expected 971')
    rows = {key: {'id': key, 'scope': scope(row.get('scope')), 'keyword': row['keyword'],
                  'original_reminder': row.get('original_reminder'), 'selection_declared': False,
                  'expected_sources': {}, 'candidate_sources': collections.defaultdict(list), 'diagnostics': []}
            for key, row in seeds.items()}
    problems = []
    for path in args.inventory:
        data = read_json(path)
        catalog = data.get('real_source_card_catalog', {})
        for selected in data.get('rows', []):
            source_id = selected.get('source_inventory_id', selected['id'])
            if source_id not in rows:
                # Coverage owns validation of real additional rules; this ledger
                # keeps their explicit inventory identity separate from seed971.
                rows[source_id] = {'id': source_id, 'scope': scope(selected.get('scope')),
                                   'keyword': selected.get('keyword'), 'selection_declared': False,
                                   'expected_sources': {}, 'candidate_sources': collections.defaultdict(list), 'diagnostics': []}
            row = rows[source_id]
            selection = selected.get('source_inventory_selection') or {}
            faces = selection.get('exact_source_faces', [])
            if selection.get('exact_source_keys') is not None:
                explicit = selection.get('authority') != 'merged_candidate_discovery'
                if not explicit and row['selection_declared']:
                    continue
                if explicit:
                    if not row['selection_declared']:
                        row['expected_sources'] = {}
                    row['selection_declared'] = True
                expected = {item['source_key']: item.get('faces', []) for item in faces}
                keys = selection['exact_source_keys']
            else:
                if row['selection_declared']:
                    continue
                keys = selected.get('real_source_keys', [])
                expected = {key: [catalog[key][field] for field in ('cardName', 'cardName2') if catalog.get(key, {}).get(field)] for key in keys}
            for key in keys:
                physical = physical_key(key, row['scope'])
                if not physical:
                    row['diagnostics'].append(f'invalid inventory physical key: {key}')
                    continue
                names = expected.get(key, expected.get(key.split(':', 1)[-1], []))
                existing = row['expected_sources'].get(physical, [])
                if names and existing and existing != names:
                    row['diagnostics'].append(f'conflicting expected face list: {physical}')
                row['expected_sources'][physical] = names or existing
    fixture_paths = set()
    for directory in args.fixtures or [root / 'fixtures/custom_mechanics']:
        directory = resolve(directory)
        fixture_paths.update(directory.glob('*.json'))
    exclusions = {}
    for path in sorted(fixture_paths):
        overlay = read_json(path)
        if overlay.get('user_decision', {}).get('authority') != 'explicit_user_scope_decision':
            continue
        for dependency in overlay.get('full_card_dependencies', []):
            namespace = scope(dependency.get('scope'))
            raw = dependency['source_card_key']
            aliases = [raw]
            full_name = dependency.get('full_name', '')
            if full_name and raw.startswith(full_name + '_'):
                aliases.extend(face + raw[len(full_name):] for face in full_name.split('//'))
            for alias in aliases:
                exclusions[physical_key(alias, namespace)] = {
                    'authority': 'explicit_user_scope_decision', 'evidence_file': display(path),
                    'evidence_sha256': digest(path), 'source_card_key': raw,
                    'mechanisms': dependency['mechanism_dependencies']}
    for path in sorted(fixture_paths):
        data = read_json(path)
        for candidate_row in data.get('mechanics', []):
            source_id = candidate_row.get('source_inventory_id', candidate_row.get('id'))
            if source_id not in rows:
                problems.append({'fixture': display(path), 'row': candidate_row.get('id'), 'reason': 'not in supplied source inventory'})
                continue
            row = rows[source_id]
            if scope(candidate_row.get('scope')) != row['scope']:
                row['diagnostics'].append(f'candidate scope mismatch: {display(path)}:{candidate_row.get("id")}')
                continue
            seed = seeds.get(source_id)
            if seed and any(candidate_row.get(field) != seed.get(field) for field in ('keyword', 'original_reminder')):
                row['diagnostics'].append(f'immutable source metadata mismatch: {display(path)}:{candidate_row.get("id")}')
                continue
            for card in candidate_row.get('representative_cards', []):
                physical = physical_key(card.get('source_card_key') or card.get('id'), row['scope'])
                if not physical:
                    row['diagnostics'].append(f'candidate without exact physical identity: {display(path)}:{card.get("name")}')
                    continue
                original = card.get('source_faces') or [{'name': card.get('name'), 'original_text': card.get('original_text')}]
                executable = card.get('executable_faces') or ([{'name': card.get('name'), 'executable_text': card.get('executable_text')}]
                                                               if card.get('executable_text') else [])
                names = [face.get('name') for face in original]
                explicit = row['expected_sources'].get(physical)
                expected_names = explicit or names
                complete = bool(expected_names) and [face.get('name') for face in executable] == expected_names and all(
                    isinstance(face.get('executable_text'), str) and face['executable_text'].strip() for face in executable)
                complete = complete and names == expected_names and all(isinstance(face.get('original_text'), str) for face in original)
                row['candidate_sources'][physical].append({'fixture': display(path), 'fixture_sha256': digest(path),
                    'candidate_row_id': candidate_row['id'], 'candidate_card_sha256': json_digest(card),
                    'original_face_names': names, 'executable_face_names': [face.get('name') for face in executable],
                    'frontend_complete': complete, 'expected_in_supplied_inventory': physical in row['expected_sources']})
    cargo = root / 'crates/ironsmith-compiler-runtime/Cargo.toml'
    registered = {item['name']: root / 'crates/ironsmith-compiler-runtime' / item.get('path', f'tests/{item["name"]}.rs')
                  for item in tomllib.loads(cargo.read_text()).get('test', [])}
    current_code = code_digest(root)
    runs = []
    run_inputs = [(path, run, None) for path in args.runs for run in read_json(path).get('runs', [])]
    for path in args.native_batch:
        run_inputs.extend((path, run, errors) for run, errors in read_native_batch(root, path))
    for path, run, prebuild_errors in run_inputs:
            reasons = list(prebuild_errors or [])
            log = resolve(run['log'])
            if not log.is_file() or digest(log) != run.get('log_sha256'):
                reasons.append('missing or changed log')
                passed = set()
            else:
                text = re.sub(r'\x1b\[[0-9;]*m', '', log.read_text())
                passed = set(re.findall(r'^test (\S+)\s+\.\.\.\s+ok\s*$', text, re.MULTILINE))
                target_runs = re.findall(r'Running tests/([^\s/]+)\.rs\b', text)
                if target_runs != [run.get('target')]:
                    reasons.append('log does not identify the declared native target')
                if not re.search(r'test result: ok\.\s+\d+ passed;\s+0 failed;', text):
                    reasons.append('no successful native test summary')
            if run.get('exit_code') != 0:
                reasons.append('missing or nonzero process exit status')
            if prebuild_errors is None and run.get('code_sha256') != current_code:
                reasons.append('missing or stale native source fingerprint')
            inputs = run.get('inputs_sha256', {})
            for name, expected in inputs.items():
                file = resolve(name)
                if not file.is_file() or digest(file) != expected:
                    reasons.append(f'stale run input: {name}')
            runs.append({**run, 'passed_cases': sorted(passed), 'validation_errors': reasons,
                         'descriptor': display(path), 'qualified': not reasons})
    bindings = []
    for path in args.bindings:
        for binding in read_json(path).get('bindings', []):
            bindings.append({**binding, 'descriptor': display(path)})
    output_rows = []
    for row in rows.values():
        sources = []
        # Candidate-only identities are reported as extras, never silently
        # accepted into the expected inventory or completion denominator.
        for key in sorted(set(row['expected_sources']) | set(row['candidate_sources'])):
            candidates = row['candidate_sources'].get(key, [])
            expected_faces = row['expected_sources'].get(key, [])
            candidate_face_sets = {tuple(candidate['original_face_names']) for candidate in candidates if candidate['frontend_complete']}
            evidence_faces = expected_faces or (list(next(iter(candidate_face_sets))) if len(candidate_face_sets) == 1 else [])
            evidence = []
            for binding in bindings:
                if binding.get('source_inventory_id') != row['id'] or physical_key(binding.get('source_key'), row['scope']) != key:
                    continue
                errors = []
                target = binding.get('target')
                test_file = registered.get(target)
                if not test_file or not test_file.is_file():
                    errors.append('native target is not registered')
                elif digest(test_file) != binding.get('test_file_sha256'):
                    errors.append('test binding changed')
                cases = binding.get('cases', [])
                if not cases:
                    errors.append('no exact strict test cases bound')
                bound_candidates = [candidate for candidate in candidates if candidate['fixture'] == binding.get('fixture')
                                    and candidate['fixture_sha256'] == binding.get('fixture_sha256')
                                    and candidate['candidate_card_sha256'] == binding.get('candidate_card_sha256')]
                if not bound_candidates or not any(candidate['frontend_complete'] for candidate in bound_candidates):
                    errors.append('no exact complete candidate matches binding hashes')
                complete_faces = bool(evidence_faces) and binding.get('faces') == evidence_faces
                if binding.get('kind') == 'strict_full_source_compile' and not complete_faces:
                    errors.append('complete selected source faces not bound')
                if not binding.get('faces') or not set(binding['faces']).issubset(set(evidence_faces)):
                    errors.append('bound source faces absent from selected source')
                routes = binding.get('routes', [])
                if not routes or any(route not in ('native', 'artifact') for route in routes):
                    errors.append('valid exercised compilation routes not declared')
                independent = binding.get('independent_native_artifact') is True and set(routes) == {'native', 'artifact'}
                if not binding.get('review_reference'):
                    errors.append('missing strict-route/source-equivalence review reference')
                if binding.get('kind') not in ('strict_full_source_compile', 'gameplay'):
                    errors.append('unknown evidence kind')
                strict_contract = complete_faces and all(binding.get(flag) is True for flag in ('strict_word_claims', 'parse_loss_free', 'complete_source_faces'))
                if binding.get('kind') == 'strict_full_source_compile' and not strict_contract:
                    errors.append('strict full-source contract not declared')
                required_inputs = {binding.get('fixture'): binding.get('fixture_sha256')}
                required_inputs.update(binding.get('additional_inputs_sha256', {}))
                for name, expected_hash in required_inputs.items():
                    input_file = resolve(name)
                    if not input_file.is_file() or digest(input_file) != expected_hash:
                        errors.append(f'changed bound input: {name}')
                if test_file:
                    required_inputs[display(test_file)] = binding.get('test_file_sha256')
                adapter = binding.get('route_adapter')
                if adapter:
                    file = resolve(adapter)
                    if not file.is_file() or digest(file) != binding.get('route_adapter_sha256'):
                        errors.append('route adapter changed')
                    required_inputs[adapter] = binding.get('route_adapter_sha256')
                qualifying = [run for run in runs if run.get('target') == target and run['qualified']
                              and set(cases).issubset(run['passed_cases'])
                              and all(run.get('inputs_sha256', {}).get(name) == value for name, value in required_inputs.items())]
                evidence.append({'target': target, 'kind': binding.get('kind'), 'cases': cases,
                                 'binding': binding['descriptor'], 'historical_case_observations': binding.get('historical_case_observations', []), 'routes': routes, 'strict_source_contract': strict_contract, 'independent_native_artifact': independent, 'validation_errors': errors,
                                 'qualifying_runs': [run['descriptor'] for run in qualifying],
                                 'current_native_pass': not errors and bool(qualifying)})
            selected = key in row['expected_sources']
            sources.append({'source_key': key, 'inventory_selected': selected and row['selection_declared'],
                            'inventory_discovered': selected, 'expected_faces': expected_faces, 'evidence_face_names': evidence_faces,
                            'in_scope': key not in exclusions, 'scope_exclusion': exclusions.get(key),
                            'candidates': candidates, 'frontend_available': any(c['frontend_complete'] for c in candidates),
                            'strict_source_verified': any(e['current_native_pass'] and e['strict_source_contract'] for e in evidence),
                            'strict_artifact_source_verified': any(e['current_native_pass'] and e['strict_source_contract'] and 'artifact' in e['routes'] for e in evidence),
                            'strict_native_source_verified': any(e['current_native_pass'] and e['strict_source_contract'] and 'native' in e['routes'] for e in evidence),
                            'independent_routes_verified': any(e['current_native_pass'] and e['independent_native_artifact'] for e in evidence),
                            'gameplay_verified': any(e['current_native_pass'] and e['kind'] == 'gameplay' for e in evidence),
                            'test_evidence': evidence})
        expected = [source for source in sources if source['inventory_discovered'] and source['in_scope']]
        output_rows.append({'id': row['id'], 'scope': row['scope'], 'keyword': row['keyword'],
                            'immutable_seed': row['id'] in seeds, 'inventory_selection_declared': row['selection_declared'],
                            'excluded_physical_sources': sum(s['inventory_discovered'] and not s['in_scope'] for s in sources),
                            'expected_physical_sources': len(expected), 'expected_faces': sum(len(s['expected_faces']) for s in expected),
                            'all_frontend_available': bool(expected) and all(s['frontend_available'] for s in expected),
                            'all_strict_source_verified': row['selection_declared'] and bool(expected) and all(s['strict_source_verified'] for s in expected),
                            'all_independent_routes_verified': row['selection_declared'] and bool(expected) and all(s['independent_routes_verified'] for s in expected),
                            'all_strict_native_verified': row['selection_declared'] and bool(expected) and all(s['strict_native_source_verified'] for s in expected),
                            'all_gameplay_verified': row['selection_declared'] and bool(expected) and all(s['gameplay_verified'] for s in expected),
                            'diagnostics': row['diagnostics'], 'sources': sources})
    result = {'schema_version': 1, 'report_only': True, 'immutable_seed_variants': 971,
              'code_sha256': current_code, 'registered_targets': sorted(registered),
              'summary': {'rows': len(output_rows), 'explicitly_selected_rows': sum(r['inventory_selection_declared'] for r in output_rows),
                          'all_strict_native_verified_rows': sum(r['all_strict_native_verified'] for r in output_rows),
                          'all_gameplay_verified_rows': sum(r['all_gameplay_verified'] for r in output_rows)},
              'unattributed_candidates': problems, 'run_evidence': runs, 'rows': output_rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    main()
