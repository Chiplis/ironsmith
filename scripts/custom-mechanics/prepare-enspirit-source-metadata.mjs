// Root-owned execution: metadata only; never changes printed source evidence.
import {readFileSync,writeFileSync,readdirSync,statSync} from 'node:fs';
import {resolve,join} from 'node:path';
const target=resolve(process.argv[2]||'fixtures/custom_mechanics');
const paths=statSync(target).isDirectory()?readdirSync(target).filter(name=>name.endsWith('.json')).map(name=>join(target,name)):[target];
for(const path of paths){
 const fixture=JSON.parse(readFileSync(path,'utf8'));if(!Array.isArray(fixture.mechanics))continue;
 const before=JSON.stringify(fixture.mechanics.map(row=>row.representative_cards?.map(card=>({original_text:card.original_text,source_faces:card.source_faces,raw_source_record:card.raw_source_record}))));
 let changed=false;
 for(const row of fixture.mechanics){
  if(String(row.keyword).toLowerCase()!=='enspirit')continue;
  if(!row.executable_definitions&&row.executable_reminder)row.executable_definitions=[{keyword:row.executable_keyword||row.keyword,reminder:row.executable_reminder}];
  for(const definition of row.executable_definitions||[]){
   if(String(definition.keyword).toLowerCase()!=='enspirit')continue;
   definition.linkedExileVerb='enspirited';definition.linkedExileScope='source';changed=true;
  }
 }
 if(before!==JSON.stringify(fixture.mechanics.map(row=>row.representative_cards?.map(card=>({original_text:card.original_text,source_faces:card.source_faces,raw_source_record:card.raw_source_record})))))throw Error(`Source evidence changed: ${path}`);
 if(changed)writeFileSync(path,JSON.stringify(fixture,null,2)+'\n');
}
