import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import StackCard from '../src/components/cards/StackCard';
import '../src/index.css';
const printing = window.__highlightPrinting;
const isYawgmoth = printing.name.startsWith('Yawgmoth');
const card = {...printing, id:10, stable_id:10, controller:0, owner:0, zone:isYawgmoth?'Battlefield':'Graveyard'};
const entry = {id:101,inspect_object_id:10,source_stable_id:10,name:card.name,controller:0,
  ability_kind:isYawgmoth?'Activated':'Triggered',source_ability_text:card.oracle_text.split('\n')[isYawgmoth?1:0],targets:[]};
const state={perspective:0,players:[{id:0,name:'You',battlefield:isYawgmoth?[card]:[],graveyard_cards:isYawgmoth?[]:[card]}],stack_objects:[entry]};
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{state}}>
  <main data-highlight-case style={{width:488,padding:24,background:'#11151a',color:'#f3ead7'}}>
    <div style={{fontFamily:'system-ui',fontSize:11,letterSpacing:2,textTransform:'uppercase',color:'#aeb6bf',marginBottom:12}}>Selected on the stack</div>
    <StackCard entry={entry} isActive onClick={()=>{}}/>
    <div style={{height:18}}/>
    <div style={{position:'relative',width:440,height:660}}>
      <HoverArtOverlay objectId={101} selectedStackEntry={entry} displayMode="card-frame" transientPreview={{card}}/>
    </div>
  </main>
</GameContext.Provider></I18nProvider>);
