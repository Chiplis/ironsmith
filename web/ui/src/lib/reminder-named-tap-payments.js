// Finite source syntax consumes only labels declared by a supplied payment recipe.
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
function label(definition){
 const match=String(definition.reminder||'').match(/(?:^|\.\s+)This generic tap payment is named ([a-z][a-z '-]{0,63})\.(?:\s|$)/i);
 return match?.[1].toLowerCase();
}
export function expandNamedTapPaymentReferences(text,definitions){
 let result=String(text);
 for(const def of definitions){
  const name=label(def);if(!name)continue;
  const body=new RegExp(`^(This (?:creature|permanent)) can't (block or be tapped|be tapped) for ${escape(def.keyword)}[.]?$`,'i');
  const rewrite=(text,join)=>{const match=String(text).trim().match(body);if(!match)return null;const restriction=`${match[1]} can't be tapped to pay the generic tap payment named ${name}.`;return match[2].toLowerCase().startsWith('block')?[`${match[1]} can't block.`,restriction].join(join):restriction;};
  result=result.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>{
   if(index%2){const rewritten=rewrite(part.slice(1,-1),`${part.at(-1)} and ${part[0]}`);return rewritten?part[0]+rewritten+part.at(-1):part;}
   return part.split('\n').map(line=>rewrite(line,'\n')??line).join('\n');
  }).join('');
 }
 return result;
}
