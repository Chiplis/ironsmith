// Materialize versioned fixture candidates through the actual frontend reader.
// Does not compile, test or certify gameplay equivalence.
import {readFileSync, readdirSync, writeFileSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {definitionsForSource, executableDefinitions, indexSourceDefinitions} from './custom-mechanics/source-definitions.mjs';
import {materializeCard, needsMissingFaces} from './custom-mechanics/cohort-input.mjs';
const path = process.argv[2];
if (!path) throw Error('usage: node scripts/materialize-custom-mechanic-cohort.mjs FIXTURE.json');
const fixture = JSON.parse(readFileSync(path, 'utf8'));
const directory = dirname(resolve(path));
const cohorts = readdirSync(directory, {withFileTypes: true})
  .filter(entry => entry.isFile() && entry.name.endsWith('.json'))
  .map(entry => join(directory, entry.name)).sort()
  .map(candidate => ({path: candidate, fixture: resolve(candidate) === resolve(path)
    ? fixture : JSON.parse(readFileSync(candidate, 'utf8'))}));
const sourceDefinitions = indexSourceDefinitions(cohorts);
for (const mechanic of fixture.mechanics) {
  // A reviewed ordinary-source correction supplies its own exact admission
  // evidence; it does not need an invented keyword definition. Explicitly
  // excluded rules and unauthored pending sources retain their old state.
  if (mechanic.support_status === 'unsupported_requires_new_rules') continue;
  for (const card of mechanic.representative_cards) {
    const supplied = executableDefinitions(mechanic, card);
    const reviewedCorrection = (Array.isArray(card.source_faces) ? card.source_faces : []).some(face =>
      Array.isArray(face?.reviewed_source_corrections)
      && face.reviewed_source_corrections.some(correction => correction?.review_status === 'source_context_reviewed'));
    if (!supplied.length && !reviewedCorrection) continue;
    try {
      if (needsMissingFaces(card)) {
        card.executable_text = null;
        delete card.executable_faces;
        card.rewrite_status = 'pending_linked_face_materialization';
        card.rewrite_error = 'Full linked-face expansion is required; this single-face cohort field cannot certify the complete card.';
        continue;
      }
      const definitions = definitionsForSource(card, supplied, sourceDefinitions, `${path}:${mechanic.id || mechanic.keyword}`);
      const faces = materializeCard(card, definitions, mechanic.scope || fixture.provenance?.scope || fixture.provenance?.format);
      const selected = faces.find(face => face.name === card.name);
      if (!selected) throw Error('Representative name does not match any complete source face');
      card.executable_faces = faces;
      Object.assign(card, selected);
      // A grant or status watcher uses the definition without gaining that
      // keyword itself. Its actual rewritten program is still audit input.
      card.rewrite_status = reviewedCorrection || faces.some(face => face.custom_keyword_identities.length
        || (typeof face.original_display_text === 'string'
          && !face.executable_text.endsWith(face.original_display_text)))
        ? 'frontend_materialized_unverified' : 'frontend_declaration_unread';
      delete card.rewrite_error;
    } catch (error) {
      card.executable_text = null;
      delete card.executable_faces;
      card.rewrite_status = error.code === 'same_source_definition_conflict' ? 'source_definition_conflict' : 'frontend_binding_failed';
      card.rewrite_error = error.message;
    }
  }
}
writeFileSync(path, JSON.stringify(fixture,null,2)+'\n');
