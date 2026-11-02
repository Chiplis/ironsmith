// Named status descriptors and explicit actors are definition data, not keywords.
const escape=value=> String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
function outside(text,offset) {return (text.slice(0,offset).match(/["“”]/g)||[]).length%2===0;}
export function expandNamedStatusCardReferences(text,definitions) {
 let result=String(text);
 for(const definition of definitions) {
  const marker=String(definition.reminder).match(/\bGive\s+[^.]*?\s+the\s+([a-z][a-z -]*?)\s+status\s+while\s+in\s+exile\b/i);
  if(!marker)continue;
  const name=marker[1].trim();const before=result;
  result=result.replace(new RegExp(`\\b${escape(name)}\\s+(cards?)\\b`,'gi'),(match,noun,offset)=>outside(before,offset)?`${noun} with the ${name} status`:match);
 }
 return result;
}
// These programs explicitly register a rule for their actor. Bind only the
// immediate instructions; the quoted rule retains its own relative "you".
export function expandPersistentPlayerActionPrograms(text,definition) {
 const body=String(definition.expansion||definition.reminder).trim().replace(/[.!?]+$/,'');
 if(!/^Exile\b/i.test(body)||! /\bRegister a persistent rule named\b/.test(body)||! /\bfor you with (?:event )?condition\b/.test(body))return text;
 const keyword=escape(definition.keyword);
 const pattern=new RegExp(`(^|[,.:;]\\s*|\\bthen\\s+)(they|that player|its owner|its controller)\\s+${keyword}(?=\\s*(?:[,.;\\n]|$))`,'gim');
 return String(text).replace(pattern,(match,prefix,actor,offset)=>{
  if(!outside(String(text),offset))return match;
  // Preserve the ordinary player antecedent supplied by the preceding action.
  let bound='',quoted=false;
  for(let index=0;index<body.length;) {
   const char=body[index];if(/["“”]/.test(char)){quoted=!quoted;bound+=char;index++;continue;}
   if(!quoted) {
    const rest=body.slice(index);const pronoun=rest.match(/^\b(your|you)\b/i);
    if(pronoun&&(index===0||!/[a-z0-9_]/i.test(body[index-1]))){bound+=pronoun[1].toLowerCase()==='your'?"that player's":'that player';index+=pronoun[0].length;continue;}
   }
   bound+=char;index++;
  }
  bound=bound.replace(/^Exile\b/i,`${actor} ${actor.toLowerCase()==='they'?'exile':'exiles'}`);
  return prefix+bound;
 });
}
