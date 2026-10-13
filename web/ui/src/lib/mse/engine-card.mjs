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

// Template choice and native style overrides belong to the caller. This layer
// maps the engine's common printable fields; it does not pick a frame by name.
export function engineCardToMse(snapshot,{faceIndex=null,rulesText,nativeCard={},set={},styling={}}={}){
  if(!snapshot||typeof snapshot!=='object'||Array.isArray(snapshot))throw new MseError('engine-card','Expected a card snapshot');
  const faces=Array.isArray(snapshot.card_faces)&&snapshot.card_faces.length?snapshot.card_faces:[snapshot];
  if(faceIndex!==null&&(!Number.isInteger(faceIndex)||faceIndex<0||faceIndex>=faces.length))throw new MseError('engine-face','Card face index is out of range');
  const selected=faceIndex===null?faces:[faces[faceIndex]];
  if(selected.length>4)throw new MseError('engine-face','More than four faces require an explicit face selection');
  const card={};
  for(let i=0;i<selected.length;i++){
    const face=selected[i],suffix=i?`_${i+1}`:'';
    const put=(name,value)=>{if(value!==undefined&&value!==null)card[name+suffix]=value;};
    put('name',plain(face.name??snapshot.name));
    put('casting_cost',engineManaToMse(face.mana_cost));
    const typeLine=String(face.type_line_display??face.type_line??'');
    const parts=typeLine.split(/\s+[—–]\s+/,2);
    put('super_type',plain(parts[0]));put('sub_type',plain(parts[1]??''));
    const rules=i===0&&rulesText!==undefined?rulesText:face.oracle_text??face.effect_text??face.ability_text??'';
    if(typeof rules!=='string')throw new MseError('engine-rules','Printable rules text must be a string');
    put('rule_text',engineRulesToMse(rules));
    put('flavor_text',plain(face.flavor_text));
    const pt=String(face.power_toughness??'').split('/');
    put('power',face.power??(pt.length===2?pt[0]:undefined));
    put('toughness',face.toughness??(pt.length===2?pt[1]:undefined));
    put('loyalty',face.defense??face.loyalty);
    if(face.rarity??snapshot.rarity)put('rarity',String(face.rarity??snapshot.rarity).replace(/^mythic$/,'mythic rare'));
  }
  Object.assign(card,nativeCard);
  return {card,set:{...set},styling:{...styling},uid:snapshot.stable_id??snapshot.id};
}
