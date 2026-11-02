// Root-only preparation through the actual source reader. Does not compile or
// certify support. Unlike the representative cohort CLI, empty local definition
// lists still consult exact source companion definitions.
import {readFileSync, readdirSync, writeFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {definitionsForSource, executableDefinitions, indexSourceDefinitions} from './source-definitions.mjs';
import {materializeCard, needsMissingFaces} from './cohort-input.mjs';
const paths = process.argv.slice(2).map(path => resolve(path));
if (!paths.length) throw Error('usage: node scripts/custom-mechanics/materialize-whole-source-audit.mjs FIXTURE.json [...]');
const protectedFields = ['original_text', 'original_source', 'original_source_record', 'raw_source_record', 'source_raw_record', 'source_faces', 'original_rules_text', 'original_printed_rules_text'];
const report = {operation: 'frontend materialization only', support_certified: false, files: [], failures: []};
for (const path of paths) {
  const fixture = JSON.parse(readFileSync(path, 'utf8'));
  const directory = dirname(path);
  const cohorts = readdirSync(directory, {withFileTypes: true})
    .filter(entry => entry.isFile() && entry.name.endsWith('.json')).map(entry => join(directory, entry.name)).sort()
    .map(candidate => ({path: candidate, fixture: candidate === path ? fixture : JSON.parse(readFileSync(candidate, 'utf8'))}));
  const index = indexSourceDefinitions(cohorts);
  const result = {path, attempted: 0, materialized: 0, failed: 0, explicitly_excluded: 0};
  for (const row of fixture.mechanics || []) {
    if (row.support_status === 'unsupported_requires_new_rules') {
      result.explicitly_excluded += (row.representative_cards || []).length;
      continue;
    }
    const supplied = executableDefinitions(row);
    for (const card of row.representative_cards || []) {
      result.attempted++;
      const saved = new Map(protectedFields.filter(key => Object.hasOwn(card, key)).map(key => [key, structuredClone(card[key])]));
      try {
        if (needsMissingFaces(card)) throw Error('Complete linked original faces are required; no sibling may be discarded.');
        const definitions = definitionsForSource(card, supplied, index, `${path}:${row.id || row.keyword}`);
        const faces = materializeCard(card, definitions, row.scope || fixture.provenance?.scope || fixture.provenance?.format);
        const selected = faces.find(face => face.name === card.name);
        if (!selected) throw Error('Representative name does not match any complete source face.');
        card.executable_faces = faces;
        Object.assign(card, selected);
        for (const [key, original] of saved) if (JSON.stringify(card[key]) !== JSON.stringify(original)) throw Error(`Immutable source field changed: ${key}`);
        card.rewrite_status = 'frontend_materialized_unverified';
        delete card.rewrite_error;
        result.materialized++;
      } catch (error) {
        for (const [key, original] of saved) card[key] = original;
        card.executable_text = null;
        delete card.executable_faces;
        card.rewrite_status = error.code === 'same_source_definition_conflict' ? 'source_definition_conflict' : 'frontend_binding_failed';
        card.rewrite_error = error.message;
        result.failed++;
        report.failures.push({path, row: row.id, source: card.source_card_key || card.id || card.name, error: error.message});
      }
    }
  }
  writeFileSync(path, `${JSON.stringify(fixture, null, 2)}\n`);
  report.files.push(result);
}
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
if (report.failures.length) process.exitCode = 1;
