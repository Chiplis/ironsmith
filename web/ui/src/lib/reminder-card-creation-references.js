// Contextual references bind declared operation receipts and exact source atoms.
const escape= value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function expandCreatedCardReferences(text,{label,verb,past,sourceName=''}) {
 const source=sourceName?`(?:this permanent|this enchantment|${escape(sourceName)})`:null;
 const singularHead=new RegExp(`^\\s*(?:When|Whenever)\\s+(?:(?:you|a player|an opponent)\\s+${verb}s?\\s+a card|a card participates in the (?:named )?${escape(label)} action performed by (?:you|a player|an opponent))\\s*,`,'i');
 return String(text).split('\n').map(line=>{
  const participant=singularHead.test(line);
  return line.split(/("[^"\n]*"|“[^”\n]*”)/g).map((part,index)=>{
   if(index%2)return part;
   // Preserve the authored object reference; a cast copy is still an object
   // even when it is not a physical card.
   const sameName=new RegExp(`\\byou(?:['’]ve| have)\\s+${past}\\s+a card with the same name as (it|that spell|that card|that permanent) this turn\\b`,'gi');
   part=part.replace(sameName,(_,object)=>`${object} is an object sharing a name with a card from your ${label} action this turn`);
   if(!source)return part;
   if(participant){
    const condition=new RegExp(`\\bits name (isn't|is not|is) noted for ${source}(?=\\s*[,.;])`,'gi');
    let ownsNote=false;
    part=part.replace(condition,(_,verb)=>{ownsNote=true;return `the triggering card's name ${verb} noted for this permanent`;});
    if(ownsNote)part=part.replace(/\byou may note its name(?=\s*[.;])/gi,"you may note the triggering card's name for this permanent");
   }
   const filter=new RegExp(`\\bwith a name noted for ${source}(?=\\s+cost\\b)`,'gi');
   part=part.replace(filter,'with a name noted for this permanent');
   return part;
  }).join('');
 }).join('\n');
}
