#!/usr/bin/env python3
"""Source-only inventory of explicit completion implementations and phase hooks.

This inventory is not a semantic verdict. It excludes literal inline cfg(test)
modules and test source files, retaining their locations as separate evidence.
"""
import ast, hashlib, json, re
from pathlib import Path

root = Path('crates/ironsmith-engine/src')
work = Path('reports/audits/ownership-work')
ns = {'re': re}
tree = ast.parse(Path('scripts/audit-effect-action-ownership.py').read_text())
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'mask'], type_ignores=[]), 'mask', 'exec'), ns)
mask = ns['mask']

def block_end(masked, opening):
    index, depth = opening + 1, 1
    while depth:
        depth += (masked[index] == '{') - (masked[index] == '}')
        index += 1
    return index

rows, excluded, scanned = [], [], {}
for path in sorted(root.rglob('*.rs')):
    source = path.read_text()
    if 'SimultaneousEffectCompletion' not in source:
        continue
    scanned[str(path)] = hashlib.sha256(source.encode()).hexdigest()
    masked = mask(source)
    test_ranges = []
    for match in re.finditer(r'#\[cfg\((?:test|ironsmith_runtime_parser_tests)\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?mod\s+\w+\s*\{', masked):
        test_ranges.append((match.start(), block_end(masked, match.end() - 1)))
    for match in re.finditer(r'impl[^{};]*\bSimultaneousEffectCompletion\s+for[^{}]+\{', masked):
        end = block_end(masked, match.end() - 1)
        body = source[match.start():end]
        is_test = 'test' in path.name or any(a <= match.start() < b for a, b in test_ranges)
        status = re.search(r'fn\s+original_phase_status\b[^{}]*\{', mask(body))
        phase_source = None
        if status:
            phase_source = body[status.start():block_end(mask(body), status.end() - 1)]
        row = {'path': str(path), 'line': source[:match.start()].count('\n') + 1,
               'implementation': ' '.join(source[match.start():match.end()].split()),
               'implementation_sha256': hashlib.sha256(body.encode()).hexdigest(),
               'original_phase_hook': phase_source,
               'status': 'Explicit phase declaration/forwarding; semantic review pending' if status else 'Default Combined; semantic review pending'}
        (excluded if is_test else rows).append(row)

report = {'complete': False, 'tests_run': False, 'builds_run': False, 'rust_edits': False,
          'scope': 'Literal explicit SimultaneousEffectCompletion implementations in current engine src, with inline test ranges and test source files separately classified.',
          'production_implementation_count': len(rows),
          'production_default_combined_count': sum(row['original_phase_hook'] is None for row in rows),
          'implementations': rows, 'excluded_test_implementations': excluded,
          'excluded_test_implementation_count': len(excluded),
          'excluded_test_default_combined_count': sum(row['original_phase_hook'] is None for row in excluded),
          'source_sha256': scanned,
          'limitations': ['Hook presence does not prove its role, forwarding, packet lifetime, freezing, source or observation contract.',
                         'This is a literal implementation inventory; generated implementations and downstream external implementors are not certified.',
                         'Default Combined remains a compatibility/safety contract and must not be removed on the strength of this inventory.',
                         'No test was executed, edited or used as gameplay evidence.']}
out = work / 'original-completion-phase-current-inventory-2026-10-08.json'
out.write_text(json.dumps(report, indent=2) + '\n')
print(len(rows), 'production completion implementations inventoried;', report['production_default_combined_count'], 'default-Combined literal production implementations; semantics remain unverified.')
