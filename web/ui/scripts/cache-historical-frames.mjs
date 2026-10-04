// Restore missing pinned fixtures, without selecting new printings.
// Run from web/ui: node scripts/cache-historical-frames.mjs
import {readFile,writeFile,access} from 'node:fs/promises';
const base=new URL('../tests/fixtures/historical-frames/',import.meta.url);
const corpus=JSON.parse(await readFile(new URL('corpus.json',base),'utf8'));
const delay=()=>new Promise(resolve=>setTimeout(resolve,150));
async function missing(path,url){
 try{await access(path);return;}catch{/* Download only missing assets. */}
 await delay();const r=await fetch(url,{headers:{'User-Agent':'IronsmithFrameAudit/1.0','Accept':'*/*'}});
 if(!r.ok)throw Error(`${r.status}: ${url}`);await writeFile(path,new Uint8Array(await r.arrayBuffer()));
}
for(const entry of corpus)for(const kind of ['normal','art_crop'])await missing(new URL(`${entry.slug}-${kind}.jpg`,base),entry.printing.image_uris[kind]);
for(const set of new Set(corpus.map(c=>c.printing.set))){
 const path=new URL(`${set}-set.json`,base);await missing(path,`https://api.scryfall.com/sets/${set}`);
 const p=JSON.parse(await readFile(path,'utf8'));await missing(new URL(`${set}.svg`,base),p.icon_svg_uri);
}
console.log('Pinned historical fixtures available:',corpus.length);
