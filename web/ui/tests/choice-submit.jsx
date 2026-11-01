import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { HoverProvider } from '../src/context/HoverContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import { ObjectSelectionProvider } from '../src/context/ObjectSelectionContext';
import SelectObjectsDecision from '../src/components/decisions/SelectObjectsDecision';
import SelectOptionsDecision from '../src/components/decisions/SelectOptionsDecision';
import '../src/index.css';
const params = new URLSearchParams(location.search);
const max = Number(params.get('max') || 1);
const min = Number(params.get('min') || 0);
const options = params.get('kind') === 'options';
const cards = [10,20,30].map(id => ({ id, stable_id:id, name:'Forest', oracle_text:'{T}: Add {G}.', owner:0 }));
cards.push({ id:40, stable_id:40, name:'Gingerbread Cabin', oracle_text:'Gingerbread Cabin enters tapped.', owner:0 });
const decision = options
  ? { kind:'select_options', min, max, description:'Choose modes', options:[{index:0,description:'Draw a card',legal:true},{index:1,description:'Gain 3 life',legal:true},{index:2,description:'Unavailable mode',legal:false}] }
  : { kind:'select_objects', min, max, description:'Choose cards', candidates:cards.map(c=>({id:c.id,name:c.name,legal:true})), ...(params.has('restricted') ? { selection_alternatives:[{min:0,max:1,candidate_indices:[3]}] } : {}) };
window.__commands = [];
const state = { perspective:0, decision, players:[{id:0,name:'Alice',library_cards:cards}] };
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{state, dispatch:cmd=>window.__commands.push(cmd)}}><HoverProvider><ObjectSelectionProvider>
  <div className="battlefield-human-decision-panel" style={{width:420,padding:16,margin:24}}>
    {options ? <SelectOptionsDecision decision={decision} canAct layout="strip"/> : <SelectObjectsDecision decision={decision} canAct layout="strip"/>}
  </div>
</ObjectSelectionProvider></HoverProvider></GameContext.Provider></I18nProvider>);
