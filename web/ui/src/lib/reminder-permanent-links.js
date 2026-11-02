import {customKeywordForms} from './keyword-forms.js';
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const outsideQuotation=(text,offset)=>(text.slice(0,offset).match(/["“”]/g)||[]).length%2===0;
export function expandPermanentLinkReferences(text,definitions){
 let output=String(text);
 for(const definition of definitions){
  if(!definition.permanentLinkRelation) continue;
  const before=output;const forms=customKeywordForms(definition);
  const names=[...new Set([forms.participle,forms.designation].filter(Boolean))].map(escape).join('|');
  output=before.replace(new RegExp(`\\b(?:${names})[ \\t]+(artifact|creature|permanent)\\b`,'gi'),(whole,noun,offset)=>outsideQuotation(before,offset)?`${noun} linked by ${definition.permanentLinkRelation} to this permanent`:whole);
 }
 return output;
}
