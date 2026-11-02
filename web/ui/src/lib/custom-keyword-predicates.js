// Only the supplied finite definition vocabulary can name a custom identity.
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
function outsideQuoted(text,index){
 for(const match of String(text).matchAll(/"(?:\\.|[^"\\])*"|“[^”]*”/g)){if(index>=match.index&&index<match.index+match[0].length)return false;}return true;
}
// A creation head's `token with ...` introduces granted abilities. It is
// not a predicate on a pre-existing object. Copy-of operands remain filters.
function predicateOperand(text,index){
 if(!outsideQuoted(text,index))return false;
 const prefix=String(text).slice(0,index);
 let start=0;for(const separator of prefix.matchAll(/[.;:\n]/g)){if(outsideQuoted(prefix,separator.index))start=separator.index+1;}
 const clause=prefix.slice(start);
 const creation=clause.match(/\bcreate\b([\s\S]*)$/i);
 return !(creation && /\btokens?(?:\s+named\s+(?:"(?:\\.|[^"\\])*"|“[^”]*”))?\s*$/i.test(creation[1]) && !/\b(?:copy|copies)\s+of\b/i.test(creation[1]));
}
function pattern(def){return new RegExp(`\\b(with|without)\\s+${escape(def.keyword)}\\b`,'gi');}
function activationPermissionPattern(def){return new RegExp(`\\byou may activate ${escape(def.keyword)} abilities from your (graveyard|hand|exile|library) as though they were in your (hand|graveyard|exile|library)\\b`,'gi');}
export function mentionsCustomKeywordPredicate(text,def){return [...String(text).matchAll(activationPermissionPattern(def))].some(match=>outsideQuoted(text,match.index)) || [...String(text).matchAll(pattern(def))].some(match=>predicateOperand(text,match.index));}
export function expandCustomKeywordPredicates(text,definitions){
 let result=String(text);for(const def of definitions){
 const permissionSource=result;
 result=result.replace(activationPermissionPattern(def),(match,from,to,offset)=>outsideQuoted(permissionSource,offset)?`You may activate abilities designated ${def.abilityDesignation || def.keyword} of cards in your ${from} as though they were in your ${to}`:match);
 const before=result;result=result.replace(pattern(def),(match,verb,offset)=>predicateOperand(before,offset)?`${verb} custom keyword "${def.keyword}"`:match);}return result;
}
