import {MseError} from './document.mjs';

// Engine/printing snapshots are plain text. Escape literal markup before adding
// MSE symbol tags so a card description cannot inject template formatting.
const plain=value=>String(value??'').replaceAll('<','\u0001');
export function engineRulesToMse(value){
  return plain(value).replace(/\{([^{}\n]+)\}/g,(_,symbol)=>`<sym>${symbol}</sym>`);
}
export function engineManaToMse(value){
  return plain(value).replace(/\{([^{}\n]+)\}/g,'$1');
}
const colorNames={W:'white',U:'blue',B:'black',R:'red',G:'green'};
function printableColors(value){
  if(value===undefined||value===null)return null;
  const values=Array.isArray(value)?value:String(value).split(/[,\s]+/);
  const found=new Set();
  for(const entry of values){
    const name=String(entry).toLowerCase();
    if(!name||name==='-'||name==='colorless'||name==='c')continue;
    const code=Object.keys(colorNames).find(code=>code.toLowerCase()===name||colorNames[code]===name);
    if(!code)throw new MseError('engine-colors',`Unknown printable color ${entry}`);
    found.add(code);
  }
  return Object.keys(colorNames).filter(code=>found.has(code));
}
function frameColor(colors,type){
  const choices=colors.map(code=>colorNames[code]);
  if(/\bArtifact\b/.test(type))choices.push('artifact');
  if(/\bLand\b/.test(type))choices.push('land');
  if(colors.length>1)choices.push('multicolor');
  return choices.join(', ')||'colorless';
}

function sagaChapters(rules){
  const roman=['I','II','III','IV','V','VI','VII','VIII','IX','X','XI','XII','XIII','XIV','XV','XVI','XVII','XVIII','XIX','XX'];
  const chapters=[],preamble=[];
  for(const line of rules.replace(/\r\n?/g,'\n').split('\n')){
    const match=/^\s*([IVXLCDM]+(?:\s*,\s*[IVXLCDM]+)*)\s+[—–-]\s+(.*)$/.exec(line);
    if(match){
      const labels=match[1].split(/\s*,\s*/).map(label=>roman.indexOf(label)+1);
      if(labels.some(label=>label===0))throw new MseError('engine-saga','Unsupported Saga chapter numeral');
      chapters.push({labels,lines:[match[2]]});
    }else if(chapters.length)chapters.at(-1).lines.push(line);
    else preamble.push(line);
  }
  if(!chapters.length)return null;
  if(chapters.length>8)throw new MseError('engine-saga','More than eight Saga ability boxes require a different template schema');
  const snaps=[];
  chapters.forEach((chapter,i)=>{for(const label of chapter.labels){
    if(snaps[label-1]!==undefined)throw new MseError('engine-saga','Duplicate Saga chapter label');
    snaps[label-1]=i+1;
  }});
  if(Array.from(snaps).some(value=>value===undefined))throw new MseError('engine-saga','Saga chapter labels must form a complete sequence');
  return {chapters,preamble:preamble.join('\n'),snaps:snaps.join(',')};
}

// Template choice and native style overrides belong to the caller. This layer
// maps the engine's common printable fields; it does not pick a frame by name.
export function engineCardToMse(snapshot,{faceIndex=null,rulesText,nativeCard={},set={},styling={}}={}){
  if(!snapshot||typeof snapshot!=='object'||Array.isArray(snapshot))throw new MseError('engine-card','Expected a card snapshot');
  const faces=Array.isArray(snapshot.card_faces)&&snapshot.card_faces.length?snapshot.card_faces:[snapshot];
  if(faceIndex!==null&&(!Number.isInteger(faceIndex)||faceIndex<0||faceIndex>=faces.length))throw new MseError('engine-face','Card face index is out of range');
  const selected=faceIndex===null?faces:[faces[faceIndex]];
  if(selected.length>4)throw new MseError('engine-face','More than four faces require an explicit face selection');
  const card={},inferredStyling={};
  for(let i=0;i<selected.length;i++){
    const face=selected[i],suffix=i?`_${i+1}`:'';
    const put=(name,value)=>{if(value!==undefined&&value!==null)card[name+suffix]=value;};
    put('name',plain(face.name??snapshot.name));
    put('casting_cost',engineManaToMse(face.mana_cost));
    const typeLine=String(face.type_line_display??face.type_line??'');
    const parts=typeLine.split(/\s+[—–]\s+/,2);
    put('super_type',plain(parts[0]));put('sub_type',plain(parts[1]??''));
    const canonicalType=String(face.type_line??typeLine);
    const indicator=printableColors(face.color_indicator);
    if(indicator!==null)put('indicator',frameColor(indicator,''));
    const signature=String(face.characteristic_signature??'').match(/^colors:([^\n]*)/m);
    const colors=printableColors(signature?signature[1]:face.colors)??indicator;
    const costColors=Object.keys(colorNames).filter(code=>String(face.mana_cost??'').includes(code));
    // Preserve template-specific hybrid/land/artifact inference when its mana
    // colors agree. Explicit live colors and indicators can override that
    // inference (e.g. devoid or a colored face without a mana cost).
    if(colors!==null&&colors.join('')!==costColors.join('')&&(!/\bLand\b/.test(canonicalType)||colors.length))
      put('card_color',frameColor(colors,canonicalType));
    const rules=i===0&&rulesText!==undefined?rulesText:face.oracle_text??face.effect_text??face.ability_text??'';
    if(typeof rules!=='string')throw new MseError('engine-rules','Printable rules text must be a string');
    let renderedRules=rules;
    if(/\bPlaneswalker\b/i.test(String(face.type_line??parts[0]))){
      // The Magic game numbers eight loyalty slots per face. Static abilities
      // still occupy a paragraph/slot, but have an empty cost box.
      const paragraphs=rules.replace(/\r\n?/g,'\n').split('\n');
      renderedRules=paragraphs.map((paragraph,index)=>{
        const match=/^\s*([+\-−–]?(?:\d+|X)):\s*(.*)$/i.exec(paragraph);
        if(index>=8)throw new MseError('engine-loyalty','The Magic template schema supports eight planeswalker paragraphs per face');
        const slot=i*8+index+1,ability=match?match[2]:paragraph;
        if(match)card[`loyalty_cost_${slot}`]=match[1].replace(/[−–]/g,'-').toUpperCase();
        // Mainframe templates use independent ability boxes, while older
        // planeswalker styles read the combined rule_text field below.
        card[`level_${slot}_text`]=engineRulesToMse(ability);
        return ability;
      }).join('\n');
    }
    if(/\bSaga\b/.test(String(face.type_line??typeLine))){
      const saga=sagaChapters(rules);
      if(saga){
        saga.chapters.forEach((chapter,index)=>{card[`level_${i*8+index+1}_text`]=engineRulesToMse(chapter.lines.join('\n'));});
        if(saga.preamble)put('chapter_text',engineRulesToMse(saga.preamble));
        inferredStyling[`chapter_textboxes${suffix}`]=['one','two','three','four','five','six','seven','eight'][saga.chapters.length-1];
        inferredStyling[`snap_chapter_to_box${suffix}`]=saga.snaps;
      }
    }
    put('rule_text',engineRulesToMse(renderedRules));
    put('flavor_text',plain(face.flavor_text));
    const pt=String(face.power_toughness??'').split('/');
    put('power',face.power??(pt.length===2?pt[0]:undefined));
    put('toughness',face.toughness??(pt.length===2?pt[1]:undefined));
    put('loyalty',face.defense??face.loyalty);
    if(face.rarity??snapshot.rarity)put('rarity',String(face.rarity??snapshot.rarity).replace(/^mythic$/,'mythic rare'));
  }
  Object.assign(card,nativeCard);
  return {card,set:{...set},styling:{...inferredStyling,...styling},uid:snapshot.stable_id??snapshot.id};
}
