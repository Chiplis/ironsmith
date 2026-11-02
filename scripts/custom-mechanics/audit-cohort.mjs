// Verify materialized frontend inputs, including identity/payment metadata.
// This structural audit never certifies semantic equivalence or gameplay.
import { readFileSync, writeFileSync, appendFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { NativeWorker } from './native-worker-client.mjs';
import {cohortLayout, sourceFaces, needsMissingFaces} from './cohort-input.mjs';

const [fixturePath, outputPath, executable] = process.argv.slice(2);
if (!executable) throw Error('usage: node audit-cohort.mjs FIXTURE OUTPUT_JSONL EXECUTABLE');
const bytes = readFileSync(fixturePath);
const fixture = JSON.parse(bytes);
process.env.AUDIT_NATIVE_WORKER = resolve(executable);
const digest = data => createHash('sha256').update(data).digest('hex');
writeFileSync(outputPath, '');
writeFileSync(`${outputPath}.provenance.json`, JSON.stringify({
  fixture: resolve(fixturePath), fixture_sha256: digest(bytes),
  executable: resolve(executable), executable_sha256: digest(readFileSync(executable)),
  semantic_gate: 'requires_behavioral_verification',
}, null, 2));
const worker = new NativeWorker();
try {
  for (const mechanic of fixture.mechanics) {
    for (const card of mechanic.representative_cards || []) {
      const row = { mechanic_id: mechanic.id, card_id: card.id, semantic_gate: 'requires_behavioral_verification' };
      if (card.rewrite_status !== 'frontend_materialized_unverified' || !card.executable_text) {
        row.status = 'pending_frontend_materialization';
      } else if (needsMissingFaces(card) || (sourceFaces(card).length > 1
          && card.executable_faces?.length !== sourceFaces(card).length)) {
        row.status = 'pending_linked_face_input';
      } else {
        try {
          row.result = await worker.run({ layout: cohortLayout(card), token: !!card.token,
            customSubtypes: card.custom_subtypes || [],
            faces: (card.executable_faces || [card]).map(face => ({name: face.name,
              parse_input: face.executable_text,
              customKeywordIdentities: face.custom_keyword_identities || [],
              customKeywordPayments: face.custom_keyword_payments || [],
              suppliedCardDefinitions: face.supplied_card_definitions || [],
              customSubtypes: face.custom_subtypes || card.custom_subtypes || [],
              customKeywordCounterDefinitions: face.custom_keyword_counter_definitions || card.custom_keyword_counter_definitions || [] })) });
          row.status = row.result.canCreate ? 'structural_pass' : 'structural_failure';
        } catch (error) { row.status = 'worker_error'; row.error = error.message; }
      }
      appendFileSync(outputPath, `${JSON.stringify(row)}\n`);
    }
  }
} finally { worker.stop(); }
