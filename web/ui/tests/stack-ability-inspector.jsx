import React, {useMemo, useState} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import {HoverProvider} from '../src/context/HoverContext';
import {DragProvider} from '../src/context/DragContext';
import FloatingCardPreview from '../src/components/right-rail/FloatingCardPreview';
import StackCard from '../src/components/cards/StackCard';
import '../src/index.css';
const lines = ['{1}: Draw a card.', '{2}: Draw a card.', 'Whenever you gain life, draw a card.'];
const card = {id:10,stable_id:10,name:'Ability Inspector Fixture',type_line:'Artifact',oracle_text:lines.join('\n'),compiled_text:lines,zone:'Battlefield',controller:0,owner:0};
const entries = lines.map((line,index)=>({id:101+index*2,inspect_object_id:10,source_stable_id:10,name:card.name,controller:0,ability_kind:index===2?'Triggered':'Activated',ability_text:'Draw a card.',source_ability_text:line,targets:[]}));
entries.push({...entries[0],id:107,source_ability_text:null});
const game = {objectDetails:async id=>{window.detailsId=String(id);return card;}};
function Fixture() {
  const [pinned,setPinned] = useState(null), [enabled,setEnabled] = useState(true);
  const state = useMemo(()=>({perspective:0,players:[{id:0,name:'Alice',battlefield:[card,{id:103,name:'Unrelated ID collision',controller:0}],hand:[],graveyard:[],exile:[]}],stack_objects:entries,decision:{kind:'priority',player:0,actions:lines.slice(0,2).map((line,index)=>({index,kind:'activate_ability',object_id:10,ability_index:index,ability_text:line,mana_payment_available:enabled&&index===0}))}}),[enabled]);
  return <GameContext.Provider value={{state,game,dispatch:()=>{}}}><HoverProvider><DragProvider>
    <button onClick={()=>setEnabled(!enabled)}>Toggle availability</button>
    <button onClick={()=>setPinned('10')}>Inspect source</button>
    <div style={{width:230,marginTop:80,marginLeft:20}}>{entries.map(entry=><StackCard key={entry.id} entry={entry} onClick={(_id,meta)=>setPinned(String(meta.stackEntry.id))}/>)}</div>
    <FloatingCardPreview pinnedObjectId={pinned}/>
  </DragProvider></HoverProvider></GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><Fixture/></I18nProvider>);
