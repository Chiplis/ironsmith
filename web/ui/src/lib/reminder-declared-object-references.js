// Authored noun aliases are ordinary current-object queries. Their names and
// controller relations are definition data, never keyword or card-name tests.
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
const plain=value=>String(value||'').normalize('NFKC').replace(/[‘’]/g,"'").replace(/\s+/g,' ').trim();
const key=value=>plain(value).toLowerCase();
export function normalizeNamedObjectReferences(value){
 if(value==null)return [];
 if(!Array.isArray(value)||value.length>16)throw Error('Named object references must be a list of at most sixteen authored references.');
 const aliases=new Map();
 for(const entry of value){
  if(!entry||typeof entry!=='object'||Array.isArray(entry)||Object.keys(entry).some(field=>!['reference','name','controller'].includes(field)))throw Error('A named object reference requires only reference, name, and controller.');
  if(typeof entry.reference!=='string'||typeof entry.name!=='string')throw Error('Named object reference and name must be strings.');
  const reference=plain(entry.reference),name=plain(entry.name),controller=entry.controller;
  if(!/^[a-z][a-z '-]{0,63}$/i.test(reference))throw Error('A named object reference must be plain words.');
  if(!name||name.length>128||/[\r\n"“”{}()[\]]/.test(name))throw Error('A named object reference requires one literal object name.');
  if(!['you','opponent','any'].includes(controller))throw Error('A named object reference requires an explicit you, opponent, or any controller.');
  const normalized={reference,name,controller},prior=aliases.get(key(reference));
  if(prior&&(key(prior.name)!==key(name)||prior.controller!==controller))throw Error(`Conflicting object reference "${reference}".`);
  aliases.set(key(reference),normalized);
 }
 return [...aliases.values()].sort((a,b)=>key(a.reference).localeCompare(key(b.reference)));
}
// Quoted rule programs are executable only after an explicit grant introducer.
// Printed names/text operands and all other quoted strings remain literal.
export function mapExecutableReferenceText(value,transform,quoted=false){
 const text=String(value);let output='',cursor=0;
 for(let index=0;index<text.length;index++){
  const single=quoted && text[index]==="'" && (index===0 || /[\s(:;,]/.test(text[index-1]));
  if(!single&&!['"','“'].includes(text[index]))continue;
  const close=single?"'":text[index]==='“'?'”':'"';
  let end=text.indexOf(close,index+1);
  if(single){while(end>=0 && /[\p{L}\p{N}]/u.test(text[end-1]||'') && /[\p{L}\p{N}]/u.test(text[end+1]||''))end=text.indexOf(close,end+1);}
  if(end<0){output+=transform(text.slice(cursor,index),quoted)+text.slice(index);return output;}
  const prefix=text.slice(cursor,index),head=prefix.split(/[.\n;]/).at(-1),tail=text.slice(end+1).split(/[.\n;]/,1)[0];
  const grant=/\b(?:has|have|gains?|with)(?:\s+(?:the following )?abilit(?:y|ies))?\s*$/i.test(head)
   && !/\b(?:named|called|name|printed\s+(?:text|word))\s*$/i.test(head)
   && !/\bin\s+(?:its|their|the(?:\s+card's)?)\s+(?:(?:printed\s+)?(?:rules|oracle)\s+text|name|printed\s+text)\b/i.test(tail);
  output+=transform(prefix,quoted)+text[index]+(grant?mapExecutableReferenceText(text.slice(index+1,end),transform,true):text.slice(index+1,end))+close;
  cursor=end+1;index=end;
 }
 return output+transform(text.slice(cursor),quoted);
}
export function expandDeclaredObjectReferences(text,definitions){
 const aliases=normalizeNamedObjectReferences(definitions.flatMap(definition=>definition.namedObjectReferences||[]))
  .sort((a,b)=>b.reference.length-a.reference.length);
 const byReference=new Map(aliases.map(alias=>[key(alias.reference),alias]));
 if(!aliases.length)return String(text);
 const pattern=new RegExp(String.raw`(?<![\p{L}\p{N}_])(?:${aliases.map(alias=>escape(alias.reference)).join('|')})(?![\p{L}\p{N}_'-])`,'giu');
 return mapExecutableReferenceText(text,(part,quoted)=>part.replace(pattern,(whole,offset)=>{
    const alias=byReference.get(key(whole)),head=part.slice(0,offset);
    if(/\b(?:named|called|name|printed\s+(?:text|word))\s*$/i.test(head))return whole;
    const relation=alias.controller==='you'?' you control':alias.controller==='opponent'?' an opponent controls':'';
    const article=/\b(?:target|each|another)\s*$/i.test(head)?'':'a ';
    const delimiter=quoted?"'":'"';
    const query=`${article}permanent${relation} named ${delimiter}${alias.name}${delimiter}`;
    return /^[A-Z]/.test(whole)&&(!head.trim()||/[.\n]\s*$/.test(head))?query[0].toUpperCase()+query.slice(1):query;
 }));
}
export const hasDeclaredObjectReference=(text,definition)=>expandDeclaredObjectReferences(text,[definition])!==String(text);
