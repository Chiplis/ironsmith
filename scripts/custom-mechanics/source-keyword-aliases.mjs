// Import spellings are source evidence, not custom-definition aliases. The
// executable definition always passes the unchanged strict keyword validator.
import {compileDraftInput, mechanicKey, reminderKey, validateMechanic} from '../../web/ui/src/lib/custom-mechanics.js';
import {customKeywordForms} from '../../web/ui/src/lib/keyword-forms.js';
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
// Quote role comes from its outer syntax. Executable granted abilities may
// contain declarations/references; printed-text/name operands remain literal.
function quoteScopes(text) {
  const scopes=[];let depth=0,head='';
  for(let index=0;index<text.length;index++) {
    const character=text[index];
    if(depth){if(character==='(')depth++;else if(character===')')depth--;continue;}
    if(character==='('){depth=1;continue;}
    if(character==='"'||character==='“') {
      const close=character==='“'?'”':'"',end=text.indexOf(close,index+1);
      if(end<0){scopes.push({start:index,end:text.length,executable:false});break;}
      const tail=text.slice(end+1).split(/[.\n]/,1)[0];
      const literal=/(?:custom\s+keyword|named|called|name|printed\s+(?:text|word))\s*$/i.test(head)
        || /\bin\s+(?:its|their|the(?:\s+card's)?)\s+(?:printed\s+)?(?:rules|oracle)\s+text\b/i.test(tail);
      const executable=!literal&&/\b(?:has|have|gains?|with)\b/i.test(head);
      scopes.push({start:index,end,executable});head+=' ';index=end;continue;
    }
    if(/[.\n]/.test(character))head='';else head+=character;
  }
  return scopes;
}
function executablePosition(text,index) {
  return quoteScopes(text).filter(scope=>scope.start<index&&index<=scope.end).every(scope=>scope.executable);
}
function outsideProtected(text, transform) {
  const scopes=new Map(quoteScopes(text).map(scope=>[scope.start,scope]));
  let result='',plain='',depth=0;
  const flush=()=>{result+=transform(plain);plain='';};
  for(let index=0;index<text.length;index++) {
    const character=text[index];
    if(depth){result+=character;if(character==='(')depth++;else if(character===')')depth--;continue;}
    if(character==='('){flush();depth=1;result+=character;continue;}
    const scope=scopes.get(index);
    if(scope){flush();const body=text.slice(index+1,scope.end);
      result+=character+(scope.executable?outsideProtected(body,transform):body)+(scope.end<text.length?text[scope.end]:'');index=scope.end;continue;}
    plain+=character;
  }
  flush();return result;
}
function closesAt(text, start) {
  let depth=1;
  for(let index=start;index<text.length;index++) {
    if(text[index]==='(')depth++;
    else if(text[index]===')' && --depth===0)return index;
  }
  return -1;
}
function declares(definition, suffix) {
  const candidate=`${definition.keyword}${suffix}`;
  try {
    const output=compileDraftInput({layout:'normal',faces:[{name:'Source declaration',oracleText:candidate}]},[definition]);
    return output.faces[0].customKeywordIdentities?.includes(mechanicKey(definition.keyword));
  } catch {return false;}
}
function aliasRepeatedActions(suffix, definition, sourceKeyword) {
  const pattern=new RegExp(`([,.;:]\\s*|\\b(?:then|and|or)\\s+)${escape(sourceKeyword)}(?![a-z0-9_!?\'-])([^()\\n]*)`, 'gi');
  return outsideProtected(suffix,part=>part.replace(pattern, (match,prefix,tail)=>
    declares(definition,tail)?`${prefix}${definition.keyword}${tail}`:match));
}
function aliasDeclarations(text, definition, sourceKeyword, evidence) {
  const pattern=new RegExp(`(^|[\\n,.;:"“—]\\s*|\\bthen\\s+|\\b(?:has|have|gains?|and)\\s+)${escape(sourceKeyword)}(?![a-z0-9_!?\'-])([^()\\n]*?)\\(`,'gim');
  let output='',cursor=0,active=false;
  for(const match of text.matchAll(pattern)) {
    const start=match.index,open=start+match[0].length-1,close=closesAt(text,open+1);
    if(close<0 || start<cursor)continue;
    if(!executablePosition(text,start+match[1].length))continue;
    if(!evidence.has(reminderKey(text.slice(open+1,close))))continue;
    const suffix=aliasRepeatedActions(match[2],definition,sourceKeyword);
    if(!declares(definition,suffix))continue;
    output+=text.slice(cursor,start)+match[1]+definition.keyword+suffix+text.slice(open,close+1);
    cursor=close+1;active=true;
  }
  return {text:output+text.slice(cursor),active};
}
function aliasReferences(text, original, canonical, spellPayment) {
  const past=escape(original.participle), keyword=escape(original.name);
  const copulas=spellPayment?'was|were':"was|were|is|are|isn't|isnt|is not|becomes?|been";
  return outsideProtected(text,part=>part
    .replace(new RegExp(`\\b(${copulas})\\s+${past}\\b`,'gi'),(_,copula)=>`${copula} ${canonical.participle}`)
    .replace(new RegExp(`\\b${past}(?=\\s+(?:spells?|cards?|creatures?|permanents?|abilities?)\\b)`,'gi'),canonical.participle)
    .replace(new RegExp(`\\b${keyword}(?=\\s+(?:costs?|abilit(?:y|ies))\\b)`,'gi'),canonical.name));
}
// An explicitly sourced action may occur only as a prospective/static
// reference on this card; it need not also be declared as an instruction.
// These closed reference heads preserve all operands and quoted literals.
function aliasProspectiveActionReferences(text, definition, sourceKeyword) {
  if (definition.actionProgram !== true) return text;
  const keyword=escape(sourceKeyword);
  return outsideProtected(text,part=>part
    .replace(new RegExp(`(\\bIf[ \\t]+(?:you|an opponent|a player)[ \\t]+would)[ \\t]+${keyword}(?=[ \\t]*(?:while[ \\t]+[^,.;\\n]+)?[ \\t]*,)`,'gi'),`$1 ${definition.keyword}`)
    .replace(new RegExp(`(\\bAs[ \\t]+you)[ \\t]+${keyword}(?=[ \\t]*,[ \\t]*you[ \\t]+may[ \\t]+choose[ \\t]+another\\b)`,'gi'),`$1 ${definition.keyword}`));
}
export function aliasSourceKeywordFaces(faces, definitions) {
  let result=faces.map(face=>({...face}));
  for(const raw of definitions) {
    if(!raw.sourceKeyword)continue;
    const definition=validateMechanic(raw); // canonical names cannot collide
    if(mechanicKey(raw.sourceKeyword)===mechanicKey(definition.keyword))continue;
    const reminders=raw.sourceReminders||[raw.sourceReminder];
    if(!reminders.length || reminders.some(reminder=>typeof reminder!=='string'||!reminder.trim()))throw Error('A source keyword alias requires exact original reminder evidence.');
    const evidence=new Set(reminders.map(reminderKey));let active=false;
    result=result.map(face=>({...face,oracleText:aliasProspectiveActionReferences(face.oracleText,definition,raw.sourceKeyword)}));
    result=result.map(face=>{const aliased=aliasDeclarations(face.oracleText,definition,raw.sourceKeyword,evidence);active||=aliased.active;return {...face,oracleText:aliased.text};});
    if(active) {
      const original=customKeywordForms({keyword:raw.sourceKeyword}),canonical=customKeywordForms(definition);
      result=result.map(face=>({...face,oracleText:aliasReferences(face.oracleText,original,canonical,/\bas an additional cost to cast\b/i.test(definition.reminder))}));
    }
  }
  return result;
}
export function sourceDisplayText(text) {
  let depth=0,result='';
  for(const character of text) {
    if(character==='(')depth++;
    else if(character===')')depth--;
    else if(!depth||character==='\n')result+=character;
    if(depth<0)throw Error('Unbalanced source reminder.');
  }
  if(depth)throw Error('Unbalanced source reminder.');
  return result.split('\n').map(line=>line.trim()).filter(Boolean).join('\n');
}
