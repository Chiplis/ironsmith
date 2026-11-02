// Operation vocabulary and official keyword classifications are explicit data.
import {customKeywordForms} from './keyword-forms.js';
import {MTG_KEYWORD_RULES} from './mtg-keyword-rules.generated.js';
// Morphology forms are only a subset of official identities (for example,
// uninflected triggered keywords need no participle entry). Classification
// comes from the complete supplied official rules vocabulary instead.
const key=value=>String(value).normalize('NFKC').replace(/\s+/g,' ').trim().toLowerCase();
function officialAbilityIdentity(value){
 const wanted=key(value);
 const matches=MTG_KEYWORD_RULES.filter(rule=>rule.kind==='keywordAbility'
  && [rule.title,...(rule.aliases||[])].some(name=>key(name)===wanted));
 return matches.length===1?key(matches[0].title):null;
}
const escape=value=>value.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
export function expandCreationKeywordTriggerBridges(text,definition){
 const forms=customKeywordForms(definition);
 const gerunds=(forms.variants||[]).filter(form=>/ing$/i.test(form));
 if(!gerunds.length)return text;
 const source=new RegExp(`\\b(?:${gerunds.map(escape).join('|')})\\s+one or more cards causes ([a-z][a-z -]*?) abilities of ([^.;\\n]+?) to trigger(?=\\s*[.;]|$)`,'gi');
 return String(text).replace(source,(original,keyword,objects)=>{
  const official=officialAbilityIdentity(keyword);
  if(!official)return original;
  return `As a player performs the ${definition.keyword.toLowerCase()} action with an amount of 1 or more, trigger each triggered ability with keyword ${official} from ${objects.trim()}`;
 });
}
