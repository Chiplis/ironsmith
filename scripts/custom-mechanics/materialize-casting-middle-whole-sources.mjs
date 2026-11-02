// Explicit source-scoped materialization for the whole-source collector.
// Authoring this runner is not execution or semantic certification.
import {readFile,writeFile} from 'node:fs/promises';
import {materializeCard} from './cohort-input.mjs';
export function materializeWholeSourceInputs(input) {
  const fixture=structuredClone(input), failures=[];
  for(const row of fixture.mechanics || []) for(const card of row.representative_cards || []) {
    const originals=JSON.stringify({text:card.original_text,faces:card.source_faces,raw:card.raw_source_record});
    try {
      const faces=materializeCard(card,card.source_scoped_executable_definitions || [],fixture.scope);
      if(faces.length!==card.source_faces.length)throw Error('Materialization changed the complete face count');
      card.executable_faces=faces;card.executable_text=faces[0].executable_text;
      for(const field of ['custom_keyword_identities','custom_keyword_payments','custom_subtypes','custom_keyword_counter_definitions','custom_token_definitions','custom_tokens']) {
        if(faces[0][field]!==undefined)card[field]=faces[0][field];else delete card[field];
      }
      card.rewrite_status='frontend_materialized_unverified';delete card.rewrite_error;
    } catch(error) {
      // A stale executable candidate cannot masquerade as this failed source.
      card.executable_text=null;delete card.executable_faces;
      card.rewrite_status='full_source_frontend_materialization_failed';card.rewrite_error=String(error.message || error);
      failures.push({source_card_key:card.source_card_key,error:card.rewrite_error});
    }
    if(JSON.stringify({text:card.original_text,faces:card.source_faces,raw:card.raw_source_record})!==originals)throw Error('Immutable original source changed');
  }
  return {fixture,failures};
}
// An explicit output is required; the original corpus is never a destination.
if(process.argv[1]?.endsWith('/materialize-casting-middle-whole-sources.mjs')) {
  const [inputPath,outputPath]=process.argv.slice(2);
  if(!inputPath||!outputPath||inputPath===outputPath)throw Error('Usage: materialize-casting-middle-whole-sources.mjs INPUT OUTPUT (distinct paths)');
  const {fixture,failures}=materializeWholeSourceInputs(JSON.parse(await readFile(inputPath,'utf8')));
  await writeFile(outputPath,JSON.stringify(fixture,null,2)+'\n');
  process.stderr.write(JSON.stringify({sources:fixture.mechanics.reduce((n,row)=>n+row.representative_cards.length,0),failures},null,2)+'\n');
  if(failures.length)process.exitCode=1;
}
