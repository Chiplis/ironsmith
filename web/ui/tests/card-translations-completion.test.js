import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source=await readFile(new URL('../src/i18n/cardTranslations.js',import.meta.url),'utf8');
for(const [type,failed,expected,calls]of [['',false,'Instantáneo',1],['',true,'Instant',1],['Instantáneo',false,'Instantáneo',0]])test(`complete official type fields ${type||'missing'} ${failed}`,async()=>{
 const fetchBefore=globalThis.fetch;let requests=0;
 globalThis.fetch=async()=>({ok:true,json:async()=>({counterspell:{name:'Contrahechizo',typeLine:type,oracleText:'Contrarresta el hechizo objetivo.'}})});
 globalThis.__translationTest={cardRouteKey:()=> 'counterspell',fetchScryfallLocalizedCardTranslation:async()=>{requests++;if(failed)throw Error('Offline');return {typeLine:'Instantáneo'};},loadGeneratedTextTranslation:async()=>null,hasTranslatedFields:()=>true,translationForFace:value=>value};
 try{
  const code=source.replace(/^import .*;\n/gm,'');
  const mod=await import(`data:text/javascript,${encodeURIComponent('const {cardRouteKey,fetchScryfallLocalizedCardTranslation,loadGeneratedTextTranslation,hasTranslatedFields,translationForFace}=globalThis.__translationTest;\n'+code+'\n//'+Math.random())}`);
  const result=await mod.loadTranslatedCardView('es',{name:'Counterspell',typeLine:'Instant',rulesText:'Counter target spell.'});
  assert.equal(result.typeLine,expected);assert.equal(result.name,'Contrahechizo');assert.equal(requests,calls);
 }finally{globalThis.fetch=fetchBefore;delete globalThis.__translationTest;}
});
