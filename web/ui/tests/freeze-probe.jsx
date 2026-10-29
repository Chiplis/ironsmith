import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import {prepareCustomCard,installCustomMechanics} from '../src/lib/custom-mechanics.js';
import {setCompiledCardNames} from '../src/lib/scryfall';
import init,{WasmGame} from '../../wasm_demo/pkg/engine.js';
import '../src/index.css';
await init();
const game=new WasmGame();
game.resetEmpty(['A','B'],20);
let calls=[];
const proxy=new Proxy(game,{get(target,key){const value=target[key];return typeof value==='function'?(...args)=>{calls.push({method:key,stage:'start',time:performance.now()});const r=value.apply(target,args);calls.push({method:key,stage:'end',time:performance.now()});return r;}:value;}});
window.freezeCalls=calls;
function Fixture(){
 const [state,setState]=useState(()=>game.uiState());const [selected,setSelected]=useState(null);const [mode,setMode]=useState('inspector');
 window.freezeSetup=async({text,reminder='Create a Treasure token.',count=0,boardText='',types=['Artifact']})=>{
  setSelected(null);game.resetEmpty(['A','B'],20);
  const p=await prepareCustomCard({faces:[{name:'Complex Custom Probe',manaCost:'{0}',cardTypes:types,power:'2',toughness:'2',oracleText:text}],customMechanics:[{keyword:'plunder',reminder}]},x=>game.previewCustomCard(x));
  installCustomMechanics(p.draft.customMechanics);setCompiledCardNames(['Complex Custom Probe']);
  const object=game.createCustomCard({draft:p.input,playerIndex:0,zoneName:'battlefield',skipTriggers:true});
  for(let i=0;i<count;i++) game.createCustomCard({draft:{faces:[{name:'Support '+i,cardTypes:['Artifact'],oracleText:boardText}]},playerIndex:0,zoneName:'battlefield',skipTriggers:true});
  window.freezeObject=object;setState(game.uiState());calls.length=0;
 };
 const actions=state.decision?.actions?.filter(a=>Number(a.object_id)===Number(window.freezeObject))||[];
 return <GameContext.Provider value={{game:proxy,state}}><button onClick={()=>setSelected(Number(window.freezeObject))}>Inspect card</button><button onClick={()=>setSelected(null)}>Close inspector</button><button onClick={()=>setMode(mode==='inspector'?'card-frame':'inspector')}>Toggle layout</button><div style={{position:'relative',width:700,height:600}}>{selected!=null&&<HoverArtOverlay objectId={selected} displayMode={mode} interactiveActions={actions}/>}</div></GameContext.Provider>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><Fixture/></I18nProvider>);
