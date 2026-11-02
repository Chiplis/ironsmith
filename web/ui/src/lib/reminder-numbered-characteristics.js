// A numbered characteristic section is printed source syntax. Each section's
// ordinary ability is independently compiled behind the selected-number gate.
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
export function expandNumberedCharacteristics(text,definitions,name='') {
  const original=String(text);
  if(!definitions.some(def=>def.numberedCharacteristics && new RegExp(`^\\s*${escape(def.keyword)}(?=[ \\t—–-])`,'im').test(original)) || !/\[shape \d+\]/i.test(original)) return original;
  const parts=original.split(/\[shape (\d+)\]/i);
  const base=parts[0].replace(/[ \t]*\[\d+\/\d+\][ \t]*$/gm,'').trimEnd();
  const clauses=[];
  const seen=new Set();
  for(let index=1;index<parts.length;index+=2){
    const number=Number(parts[index]);
    if(!Number.isSafeInteger(number)||seen.has(number)) throw Error('Numbered characteristics require distinct section numbers.');
    if(number===0) throw Error('A zero-numbered section requires explicit unchosen-state evidence; this adapter supports positive section labels.');
    seen.add(number);
    const body=parts[index+1].trim();
    const pt=body.match(/\[(\d+)\/(\d+)\][ \t]*$/);
    if(!pt) throw Error(`Every numbered characteristic section needs explicit power and toughness: ${JSON.stringify(body)}`);
    const lines=body.slice(0,pt.index).trim().split('\n').map(line=>line.trim()).filter(Boolean);
    const condition=`the last number chosen for this permanent is ${number}`;
    clauses.push(`As long as ${condition}, this permanent has base power and toughness ${pt[1]}/${pt[2]}.`);
    for(let line of lines){
      // Remove explanatory reminders; the ordinary ability itself is retained.
      line=line.replace(/\([^()]*\)/g,'').trim();
      if(!line) continue;
      line=line.replace(new RegExp(`\\b(?:${name?escape(name)+'|':''}this creature|this permanent|it) takes this shape`,'gi'),'a number is chosen for this permanent');
      if(/["“”]/.test(line)) throw Error('Nested quoted numbered-characteristic abilities require an explicit ordinary grant.');
      clauses.push(`As long as ${condition}, this permanent has "${line.replace(/[.!?]+$/,'')}."`);
    }
  }
  return [base,...clauses].join('\n');
}
