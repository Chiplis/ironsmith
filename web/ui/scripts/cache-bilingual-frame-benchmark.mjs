// Populate a pinned, offline full-card corpus. Re-running preserves existing fixtures.
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const base=fileURLToPath(new URL('../tests/fixtures/bilingual-frames/',import.meta.url));
await mkdir(base,{recursive:true});
const seeds=[['Ornithopter','dmr'],['Yawgmoth, Thran Physician','dmr'],['Sigarda, Host of Herons','avr'],['Counterspell','dmr'],['Swords to Plowshares','dmr'],['Lightning Bolt','m11'],['Sylvan Library','dmr'],['Swamp','m11'],['Force of Will','dmr'],['Vampiric Tutor','dmr'],['Birds of Paradise','dmr'],['Wrath of God','dmr'],['Urza, Lord High Artificer','dmr'],['Entomb','dmr'],['Royal Assassin','dmr'],['High Tide','dmr'],['Opt','m21'],['Cultivate','m21'],['Solemn Simulacrum','m21'],['Scavenging Ooze','m21'],['Heroic Intervention','m21'],['Baneslayer Angel','m21'],['Terror of the Peaks','m21'],["Teferi's Tutelage",'m21'],['Golos, Tireless Pilgrim','m20'],['Field of the Dead','m20'],['Agent of Treachery','m20'],['Risen Reef','m20'],['Omnath, Locus of the Roil','m20'],['Leyline of Anticipation','m20'],['Cavalier of Thorns','m20'],['Dungeon Geists','m20'],['Thorn Lieutenant','m19'],['Elvish Clancaller','m19'],['Reclamation Sage','m19'],['Dismissive Pyromancer','m19'],['Satyr Enchanter','m19'],["Ajani's Pridemate",'m19'],['Death Baron','m19'],['Crucible of Worlds','m19'],['Guttersnipe','m19'],['Meteor Golem','m19'],["Stitcher's Supplier",'m19'],['Exclusion Mage','m19'],['Cleansing Nova','m19'],['Resplendent Angel','m19'],["Sarkhan's Unsealing",'m19'],['Poison-Tip Archer','m19']];
let existing=[];try{existing=JSON.parse((await readFile(base+'corpus.js','utf8')).replace(/^export default /,'').trim().replace(/;$/,''));}catch{}
const get=async url=>{await new Promise(r=>setTimeout(r,160));const r=await fetch(url,{headers:{"User-Agent":"Ironsmith-Frame-Benchmark/1.0","Accept":"application/json,image/*,*/*"}});if(!r.ok)throw Error(`${r.status} ${url}`);return r;};
const entries=[...existing];
for(const [name,set]of seeds){
 const slug=name.toLowerCase().replace(/[^a-z0-9]+/g,'-');
 let english=existing.find(c=>c.slug===slug+'-en'&&c.printing.set===set)?.printing;
 if(!english){const q=`!"${name}" set:${set} lang:en -frame:1993 -frame:1997 -is:fullart -border:borderless -frame:showcase -frame:extendedart`;english=(await(await get('https://api.scryfall.com/cards/search?'+new URLSearchParams({q,order:'released',dir:'desc'}))).json()).data[0];}
 for(const lang of ['en','es']){
  const key=slug+'-'+lang,old=existing.find(c=>c.slug===key&&c.printing.set===set);
  if(old)continue;
  const printing=lang==='en'?english:await(await get(`https://api.scryfall.com/cards/${english.set}/${english.collector_number}/es`)).json();
  for(const variant of ['normal','art_crop'])await writeFile(base+key+'-'+variant+'.jpg',Buffer.from(await(await get(printing.image_uris[variant])).arrayBuffer()));
  const setData=await(await get(`https://api.scryfall.com/sets/${printing.set}`)).json();
  await writeFile(base+printing.set+'.svg',Buffer.from(await(await get(setData.icon_svg_uri)).arrayBuffer()));
  entries.push({slug:key,printing,setSymbol:setData.icon_svg_uri});
  await writeFile(base+'corpus.js','export default '+JSON.stringify(entries,null,2)+';\n');
  console.log(key,printing.set,printing.collector_number);
 }
}
await writeFile(base+'corpus.js','export default '+JSON.stringify(entries,null,2)+';\n');
