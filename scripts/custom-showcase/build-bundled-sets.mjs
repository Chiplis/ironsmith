// Rebuild from the immutable audited exports, not from live card text.
// node scripts/custom-showcase/build-bundled-sets.mjs [source-directory] [revolution-image-catalog.json]
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import { executableDefinitions, indexSourceDefinitions, definitionsForSource } from '../custom-mechanics/source-definitions.mjs';
import { aliasSourceKeywordFaces, sourceDisplayText } from '../custom-mechanics/source-keyword-aliases.mjs';
import { mergeMechanics } from '../../web/ui/src/lib/custom-mechanics.js';
import { sourceDraftFaces } from '../custom-mechanics/cohort-input.mjs';
import { sourceSubtypeDefinitions } from '../custom-mechanics/source-subtypes.mjs';
import { mergeSubtypes } from '../../web/ui/src/lib/custom-subtypes.js';
const sourceDir = process.argv[2] || 'scripts/custom-showcase/sources';
const imagePath = process.argv[3] || 'scripts/custom-showcase/sources/revolution-images.json.gz';
const out = 'web/ui/public/custom-sets';
fs.mkdirSync(out, {recursive:true});
const readSource = file => { const candidate=fs.existsSync(file)?file:`${file}.gz`; const bytes=fs.readFileSync(candidate);return candidate.endsWith('.gz')?gunzipSync(bytes):bytes; };
const migratedMsemSets = new Set(['SHU','G5L','LIP','AEK','LRH','MSY']);
const imageRows = JSON.parse(readSource(imagePath)).cards;
const imageIndex = new Map(imageRows.map(c => [`${c.set}:${c.card_name.replace(new RegExp(`_${c.set}$`),'')}`, c]));
const rowsBySet = rows => {const result=new Map(); for(const row of rows) {if(!result.has(row.set))result.set(row.set,[]);result.get(row.set).push(row);}return result;};
const revBySet=rowsBySet(imageRows);
const fixtures = fs.readdirSync('fixtures/custom_mechanics').filter(f=>f.endsWith('.json')).map(f=>({path:`fixtures/custom_mechanics/${f}`,fixture:JSON.parse(fs.readFileSync(`fixtures/custom_mechanics/${f}`))}));
const definitionIndex = indexSourceDefinitions(fixtures); // recovery fallback only; canonical rows take priority
const endgate = JSON.parse(fs.readFileSync('fixtures/custom_mechanics/custom-source-endgate.json'));
const byPath = new Map(fixtures.map(f=>[f.path,f.fixture]));
const candidates = new Map();
for (const entry of endgate.cases) {
 const mechanic = byPath.get(entry.fixture)?.mechanics?.find(m=>m.id===entry.row_id);
 const card = mechanic?.representative_cards?.find(c=>c.id===entry.record_id);
 if (!card) continue;
 const prior=candidates.get(entry.physical_key);
 if (!prior || (!prior.card.executable_faces?.length && card.executable_faces?.length)) candidates.set(entry.physical_key,{card,mechanic,entry});
}
const clean = text => String(text||'').replace(/(?<![\p{L}\p{N}*/])\*([^*\n]+)\*(?![*/])/gu,'$1').replace(/\r/g,'');
const supers = new Set(['Basic','Legendary','Snow','World','Ongoing']);
function types(line) {
 const [head,tail='']=String(line||'').split(/\s+[—–]\s+/);
 const words=head.trim().split(/\s+/).filter(Boolean);
 return {supertypes:words.filter(t=>supers.has(t)),cardTypes:words.filter(t=>!supers.has(t)),subtypes:tail.trim().split(/\s+/).filter(Boolean)};
}
const layout = {normal:'single',doubleface:'transform',transform:'transform',transform_like:'modal_dfc',split:'split',aftermath:'split',adventure:'adventure',flip:'flip',prepare:'prepare'};
const searchEntries=[];
const manifest = {version:1, sources:[], sets:[], cardCount:0, missingImages:[], preparationIssues:[]};
for (const scope of ['msem','rev']) {
 const file=path.join(sourceDir,`${scope}-complete-source.json`),bytes=readSource(file),source=JSON.parse(bytes);
 manifest.sources.push({scope,url:`https://lackeybot.com/${scope}/search`,sha256:crypto.createHash('sha256').update(bytes).digest('hex')});
 const setNames=new Map();
 for(const print of Object.values(source.prints)) for(const [name,rows] of Object.entries(print)) for(const r of rows) setNames.set(r.setID,name);
 const sets=new Map();
 for(const raw of source.cards) {
  const key=`${scope}:${raw.key}`, setCode=raw.setID;
  if(!sets.has(setCode)) sets.set(setCode,{id:`${scope}:${setCode}`,name:`${scope==='msem'?'MSEM':'Revolution'} · ${setNames.get(setCode)||setCode}`,code:setCode,source:{scope,url:`https://lackeybot.com/${scope}/search`,setName:setNames.get(setCode)||setCode},cards:[],customMechanics:[],customSubtypes:[]});
  const set=sets.get(setCode), selected=candidates.get(key);
  let definitions=[], issue=null;
  try { if(selected) definitions=definitionsForSource(selected.card,executableDefinitions(selected.mechanic,selected.card),definitionIndex,selected.entry.fixture); }
  catch(e) {
    try { definitions=definitionsForSource(selected.card,executableDefinitions(selected.mechanic,selected.card),new Map(),selected.entry.fixture); }
    catch(failure) { issue=failure.message; }
  }
  const attribution={sourceFormat:scope,setCode,setName:set.source.setName,collectorNumber:String(raw.cardID),artist:raw.artist||'',designer:raw.designer||'',sourceCardKey:raw.key,sourceUrl:`https://lackeybot.com/${scope}/search?q=${encodeURIComponent(`e:${setCode} ${raw.cardName}`)}`};
  definitions=definitions.map(def=>({...def, sourceCard:raw.cardName, attribution}));
  const comparable = name => String(name).replace(new RegExp(`_${setCode}$`),'').replace(/[^\p{L}\p{N}]/gu,'').toLowerCase();
  const matchingPrints=(revBySet.get(setCode)||[]).filter(c=> String(c.number)===String(raw.cardID) && !String(c.shape).includes('token'));
  const image = imageIndex.get(`${setCode}:${raw.cardName}`) || matchingPrints.find(c=>comparable(c.card_name)===comparable(raw.fullName)) || (matchingPrints.length===1 ? matchingPrints[0] : null);
  if (scope==='rev' && image?.artist) { attribution.originalArtist=attribution.artist; attribution.artist=image.artist; }
  let imageUrl=scope==='msem'?`https://mse-modern.com/msem2/images/${encodeURIComponent(setCode)}/${encodeURIComponent(raw.cardID)}.jpg`:image?`https://escapefromlimbo.github.io/sets/${encodeURIComponent(setCode)}-files/img/${image.number}_${encodeURIComponent(image.card_name)}${image.shape.includes('double')?'_front':''}.${image.image_type}`:'';
  if(scope==='rev' && image?.image_file) imageUrl=`https://escapefromlimbo.github.io/sets/${encodeURIComponent(setCode)}-files/img/${encodeURIComponent(image.image_file)}`;
  if(scope==='msem' && migratedMsemSets.has(setCode)) imageUrl=`https://raw.githubusercontent.com/cajunwritescode/MSEM/refs/heads/main/img/${encodeURIComponent(setCode)}/${encodeURIComponent(raw.cardID)}${raw.shape==='doubleface'?'a':''}.jpg`;
  if(!imageUrl)manifest.missingImages.push(key);
  const faces=['',...(raw.cardName2?['2']:[])].map((suffix,index)=>({
   name:raw[`cardName${suffix}`],manaCost:raw[`manaCost${suffix}`]||'',...types(raw[`typeLine${suffix}`]),
   oracleText:clean(raw[`rulesText${suffix}`]), originalPrintedRulesText:String(raw[`rulesText${suffix}`]||''),
   power:raw[`power${suffix}`]||'',toughness:raw[`toughness${suffix}`]||'',loyalty:raw[`loyalty${suffix}`]||'',defense:raw[`defense${suffix}`]||'',
   colorIndicator:[], flavorText:clean(raw[`flavorText${suffix}`]),
   attribution:{...attribution,artist:index?(image?.artist2||raw.artist2||attribution.artist):attribution.artist},
   imageUrl:index&&scope==='msem'&&raw.shape==='doubleface'?imageUrl.includes('raw.githubusercontent.com')?imageUrl.replace(/a\.jpg$/,'b.jpg'):imageUrl.replace(/\.jpg$/,'b.jpg'):index&&scope==='rev'&&image?.shape.includes('double')?imageUrl.replace('_front.','_back.'):imageUrl,
  }));
  // Retain source print separately. Editable oracle text uses the same scoped
  // definition expansion as the set editor, rather than baking expanded prose.
  let prepared=faces;
  if(selected) {
   try {
    const reviewed=sourceDraftFaces(selected.card);
    if(reviewed.length===faces.length)prepared=faces.map((face,index)=>({...face,oracleText:reviewed[index].oracleText}));
   } catch(error) { issue=error.message; }
  }
  if(definitions.length) {
   try {prepared=aliasSourceKeywordFaces(prepared,definitions).map(f=>({...f,oracleText:sourceDisplayText(f.oracleText)}));}
   catch(e){issue=e.message;definitions=[];prepared=faces;}
  }
  const draft={customMechanicsScope:'card',layout:layout[raw.shape]||raw.shape||'single',hasFuse:/\bFuse\b/.test(raw.rulesText||''),faces:prepared,attribution,customMechanics:definitions,customSubtypes:mergeSubtypes(selected?.card.custom_subtypes||[],sourceSubtypeDefinitions({}, {faces:prepared.map((f,i)=>({...f,typeLine:raw[i?'typeLine2':'typeLine']}))}, scope)),...(selected?.card.supplied_card_definitions?.length?{suppliedCardDefinitions:selected.card.supplied_card_definitions}:{}),...(selected?.card.custom_tokens?.length?{customTokens:selected.card.custom_tokens}:{})};
  // A set owns one definition per keyword. Keep conflicts visible on the card
  // as import issues; never silently substitute a different meaning.
  try {set.customSubtypes=mergeSubtypes(set.customSubtypes,draft.customSubtypes);}
  catch(e){issue=e.message;}
  // Definitions remain card-scoped; the workspace presents distinct variants.
  set.customMechanics=[];
  if(issue)manifest.preparationIssues.push({key,reason:issue});
  set.cards.push({id:`bundled:${key}`,draft,source:attribution,...(issue?{importIssue:issue}:{}),...((selected?.entry.exclusion_proofs?.length || ['Tunu, Ancient Shaper','Glitch, Cosmic Dreamer'].includes(raw.cardName))?{showcaseExcluded:true}:{})});
 }
 for(const set of sets.values()) {
  for(const card of set.cards) searchEntries.push({id:card.id,setId:set.id,names:card.draft.faces.map(face=>face.name)});
  set.source.designers=[...new Set(set.cards.map(card=>card.draft.attribution.designer).filter(Boolean))];
  const filename=`${scope}-${encodeURIComponent(set.code)}.json`;
  fs.writeFileSync(path.join(out,filename),JSON.stringify(set));
  manifest.sets.push({id:set.id,name:set.name,code:set.code,source:set.source,cardCount:set.cards.length,bundlePath:`custom-sets/${filename}`,cards:[]});
  manifest.cardCount+=set.cards.length;
 }
}
fs.writeFileSync(path.join(out,'search.json'),JSON.stringify(searchEntries));
fs.writeFileSync(path.join(out,'index.json'),JSON.stringify(manifest));
console.log(JSON.stringify({sets:manifest.sets.length,cards:manifest.cardCount,missingImages:manifest.missingImages.length,preparationIssues:manifest.preparationIssues.length}));
