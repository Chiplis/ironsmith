export function registeredPrintedText(field) {
  if(field.printedTextOverride)return field.printedTextOverride;
  if(field.kind==='flavor')return field.text.replace(/\*([^*]+)\*/g,'$1');
  if(field.kind==='name'){if(field.headingAlias)return field.printedText;const printed=(field.lines||[]).map(l=>l.text).join(' ');return printed.toLowerCase()===field.text.toLowerCase()?printed:field.text;}
  if(!['rule','type'].includes(field.kind))return field.text;
  let printed=(field.lines||[]).map(l=>field.loyaltyCost?l.text.replace(/^\(?\s*[:.•]\s*/,''):l.text).join(' ').replace(/([,;:])(?=[A-Za-z])/g,'$1 ');
  if(!printed)return field.text;
  if(/^\(\{T\}: Add (?:\{[^}]+\}|[,or\s])+\.\)$/.test(field.text))return field.text;
  if(field.kind==='type'){
    const tokens=s=>(s.match(/[A-Za-z]+/g)||[]).map(w=>w.toLowerCase()).sort().join(' ');
    if(tokens(printed)===tokens(field.text))return printed.replace(/\s[-–]\s/g,' — ');
  }
  if(field.text.startsWith('(')&&field.text.endsWith(')')){
    if(!printed.startsWith('('))printed='('+printed;
    if(!printed.endsWith(')'))printed+=')';
  }
  // OCR sometimes runs adjacent printed words together. Restore those spaces
  // only when the same adjacent words are present in the known card text.
  const words=field.text.replace(/\{[^}]+\}/g,' ').match(/[A-Za-z]+/g)||[];
  // Repair a single OCR substitution only when the known source vocabulary
  // supplies one unambiguous spelling. Historical wording remains intact.
  const vocabulary=[...new Set(words.map(word=>word.toLowerCase()))];
  const oneEdit=(a,b)=>{
    if(Math.abs(a.length-b.length)>1)return false;
    let i=0,j=0,edits=0;
    while(i<a.length&&j<b.length){if(a[i]===b[j]){i++;j++;continue;}if(++edits>1)return false;if(a.length>=b.length)i++;if(b.length>=a.length)j++;}
    return edits+a.length-i+b.length-j===1;
  };
  printed=printed.replace(/[\p{L}]+/gu,observed=>{
    const plain=observed.normalize('NFKD').replace(/[\u0300-\u036f]/g,'').replace(/ı/g,'i').toLowerCase();
    if(plain.length<4)return observed;
    const candidates=vocabulary.includes(plain)?[plain]:vocabulary.filter(word=>word.length>=4&&oneEdit(plain,word));
    if(candidates.length!==1)return observed;
    const word=candidates[0];return observed===observed.toUpperCase()?word.toUpperCase():/^[A-Z]/.test(observed)?word[0].toUpperCase()+word.slice(1):word;
  });
  for(let i=0;i<words.length-1;i++)for(let count=2;count<=4&&i+count<=words.length;count++) {
    const sequence=words.slice(i,i+count),joined=sequence.join('');
    if(joined.length>5)printed=printed.replace(new RegExp('\\b'+joined+'\\b','gi'),observed=>observed===observed.toUpperCase()?sequence.join(' ').toUpperCase():sequence.join(' '));
  }
  if(field.loyaltyCost||field.inlineLoyalty) {
    printed=printed.replace(/^[^A-Za-z“"(]+/, '').replace(/^\s*:\s*/, '');
    printed=(field.loyaltyCost||field.text.match(/^([+−-]?\d+):/)[1])+': '+printed;
  }
  // Restore an ability word's italic separator only when it is actually
  // present on the printing; older printings may omit the ability word.
  const abilityWord=field.text.replace(/^[IVX]+ — /,'').match(/^([A-Za-z][A-Za-z '’-]+) — /);
  if(abilityWord)printed=printed.replace(new RegExp('^'+abilityWord[1]+'\\s*(?:[-–—]\\s*)?','i'), ()=>abilityWord[1]+' — ');
  for(const reduction of field.text.matchAll(/\b(costs?)\s+((?:\{[^}]+\})+)\s+less/gi))
    printed=printed.replace(/\b(costs?)\s+[^\s]+\s+less/gi, (_,word)=>word+' '+reduction[2]+' less');
  for(const ward of field.text.matchAll(/\bward\s+((?:\{[^}]+\})+)/gi))printed=printed.replace(/\bward\s+\S+/gi, observed=>observed.split(/\s/)[0]+' '+ward[1]);
  for(const stats of field.text.matchAll(/([+−-]\d+)\/([+−-]\d+)/g)) {
    const escapedSign=value=>value.replace(/\+/g,'\\+').replace(/[−-]/g,'[−-]');
    const a=escapedSign(stats[1]),b=escapedSign(stats[2]);
    printed=printed.replace(new RegExp(a+'[lI1|/]'+b,'g'),stats[0]);
  }
  if(field.prototypeRail)printed=printed.replace(/^Prototype\s+[^(]+(?=\()/i,'Prototype ').replace(/\)\s+\d+\/\d+$/,' )');
  if(field.text.includes('{T}: Add'))printed=printed.replace(/([“"(])\s*[^\s{}:]+:\s*Add/gi, '$1{T}: Add');
  const manaOutput=field.text.match(/\bAdd\s+((?:\{[^}]+\}(?:[,or\s]*))+)/i);
  if(manaOutput&&!/\{T\}: Add/.test(field.text)&&!/one colorless mana/i.test(printed))printed=printed.replace(/\bAdd\s+[^.]+/i,observed=>{
    const tail=observed.slice(4).match(/\b(?:Exile|Then|You|This|When|Sacrifice|Draw)\b[\s\S]*$/);
    return observed.match(/^Add/i)[0]+' '+manaOutput[1].trim()+(/to your mana pool/i.test(observed)?' to your mana pool':'')+(tail?'. '+tail[0]:'');
  });

  // Preserve printed prose, including historical wording and omitted ability
  // words. OCR cannot read mana pips reliably: use the known cost where the
  // printed field and oracle have the same activated/level cost.
  if(/^\{[^}]+\}.*: Add /.test(field.text)) {
    if(/one colorless mana/i.test(printed))return field.text.split(':')[0]+': '+printed.slice(printed.search(/\bAdd\b/i));
    const historical=/to your mana pool/i.test(printed)?' to your mana pool':'';
    const firstSentence=field.text.slice(0,field.text.indexOf('.')<0?undefined:field.text.indexOf('.'));
    const canonicalTail=field.text.slice(field.text.indexOf('.')+1).trim();
    const poolTail=printed.match(/to your mana pool\.([\s\S]*)$/i);
    const printedTail=canonicalTail?(poolTail?poolTail[1]:printed.includes('.')?printed.slice(printed.indexOf('.')+1):''):'';
    const result=firstSentence+historical+'.'+printedTail;
    return /[A-Z]/.test(printed)&&printed===printed.toUpperCase()?result.toUpperCase():result;
  }
  const cost=field.text.match(/^(.*?):\s*/);
  if(cost&&/^\{[^}]+\}/.test(cost[1])) {
    if(/: Level [23]$/.test(field.text))return field.text;
    if(printed.includes(':')) {
      const observedCost=printed.split(':')[0];
      const prose=cost[1].match(/\b(?:Sacrifice|Pay|Discard|Remove|Tap|Exile)\b/i);
      const observedProse=prose&&observedCost.match(new RegExp('\\b'+prose[0]+'\\b.*','i'));
      const restoredCost=observedProse?cost[1].slice(0,prose.index)+observedProse[0]:cost[1];
      return restoredCost+': '+printed.split(':').slice(1).join(':').trim();
    }
  }
  const keywordCost=field.text.match(/^(.*?)((?:\{[^}]+\})+)(.*)$/);
  // Inline payment pips are printed symbols too. Match the nearby payment
  // word, including Vision's common Y/V confusion in all-capital lettering.
  for(const match of field.text.matchAll(/\b(pays?|paying)\s+((?:\{[^}]+\})+)/gi)) {
    const payment=match[1];
    const pattern=payment.replace(/y/gi,'[yv]');
    printed=printed.replace(new RegExp('\\b'+pattern+'\\s+((?:an?\\s+)?(?:additional\\s+)?)'+'[^\\s.,;:]+','gi'), (observed,qualifier) => {
      const word=observed.slice(0,observed.indexOf(' '));
      return (word===word.toUpperCase()?payment.toUpperCase():payment)+' '+qualifier+match[2];
    });
  }
  const inlineMana=field.text.match(/\{T\}: Add ((?:\{[^}]+\}(?:[,or\s]*))+)/);
  if(field.text.includes('{T}: Add '))printed=printed.replace(/(?:\{T\}|[^\s(“"{}:]+)\s*:\s*Add/g,'{T}: Add');
  if(inlineMana&&!/^\{T\}: Add /.test(field.text)) {
    printed=printed.replace(/(?:\{T\}|[^\s(“"{}]+):\s*Add\s+[^.)”"]+/i, observed=>'{T}: Add '+inlineMana[1].trim()+(/to your mana pool/i.test(observed)?' to your mana pool':''));
  }
  if(!field.prototypeRail&&keywordCost&&keywordCost[1].trim().split(/\s+/).length<=3&&/^[A-Za-z -]+\s$/.test(keywordCost[1])&&!/\b(?:pays?|paying)\s*$/i.test(keywordCost[1])&&printed.toLowerCase().startsWith(keywordCost[1].toLowerCase())) {
    let remainder=printed.includes('(')?printed.slice(printed.indexOf('(')):'';
    const reminderCost=keywordCost[3].match(/\(((?:\{[^}]+\})+):/);
    if(!remainder&&reminderCost&&/\bPut a level counter\b/.test(printed))remainder='('+reminderCost[1]+': '+printed.slice(printed.indexOf('Put a level counter'));
    if(reminderCost&&remainder.includes(':'))remainder='('+reminderCost[1]+':'+remainder.split(':').slice(1).join(':');
    return printed.slice(0,keywordCost[1].length)+keywordCost[2]+(remainder?' '+remainder:'');
  }
  const activated=field.text.match(/^((?:\{[^}]+\}(?:,?\s*)?)+):\s*/);
  if(activated) {
    const effect=field.text.slice(activated[0].length).match(/^[A-Za-z]+/)?.[0];
    const start=effect?printed.indexOf(effect):-1;
    if(start>=0)return activated[1]+': '+printed.slice(start);
  }
  return printed;
}
