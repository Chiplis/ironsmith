// A comma list of the same supplied bare declaration denotes independent instances.
const escape=value=>String(value).replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
export function expandRepeatedBareDeclarations(text,definitions){
 let result=String(text);
 for(const def of definitions){
  if(def.template||/\{(?:Cost|Number|Object|Subject|Type)\}/i.test(def.reminder||''))continue;
  const name=escape(def.keyword),pattern=new RegExp(`^([ \t]*)(${name})(?:[ \t]*,[ \t]*${name})+[ \t]*[.]?$`,'gim');
  result=result.replace(pattern,(line,space)=>line.trim().replace(/[.]$/,'').split(',').map(()=>space+def.keyword).join('\n'));
 }
 return result;
}
// Brackets explicitly bound a complete repeated program, including its own nested quotes.
export function boundedSourceProgram(text){
 let output='',quoted=false,depth=0;
 for(const ch of String(text).trim().replace(/[.]$/,'')){
  if(ch==='"'||ch==='“'||ch==='”')quoted=!quoted;
  if(!quoted){if(ch==='[')depth++;if(ch===']')depth--;}
  output+=ch==='.'&&!quoted&&depth===0?';':ch;
 }
 return `[${output}]`;
}
