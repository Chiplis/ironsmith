import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { DragProvider } from '../src/context/DragContext';
import { CombatArrowProvider } from '../src/context/CombatArrowContext';
import { HoverProvider } from '../src/context/HoverContext';
import { ObjectSelectionProvider } from '../src/context/ObjectSelectionContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import { TooltipProvider } from '../src/components/ui/tooltip';
import SelectObjectsDecision from '../src/components/decisions/SelectObjectsDecision';
import SelectOptionsDecision from '../src/components/decisions/SelectOptionsDecision';
import TargetsDecision from '../src/components/decisions/TargetsDecision';
import FloatingCardPreview from '../src/components/right-rail/FloatingCardPreview';
import '../src/index.css';

const cards = [
  { id: 1, name: 'Eternal Witness', owner: 0, controller: 0, zone: 'graveyard', legal: true },
  { id: 2, name: 'Swords to Plowshares', owner: 1, controller: 1, zone: 'exile', legal: true },
  { id: 3, name: 'Rhystic Study', type_line: 'Enchantment', mana_cost: '{2}{U}', colors: ['U'], oracle_text: 'Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.', owner: 0, controller: 1, zone: 'battlefield', legal: true },
  { id: 4, name: 'Sol Ring', owner: 2, controller: 2, zone: 'battlefield', legal: true },
  { id: 5, name: 'Counterspell', owner: 0, controller: 0, zone: 'hand', legal: true },
  { id: 6, name: 'Atraxa, Praetors’ Voice', owner: 3, controller: 3, zone: 'command', legal: true },
  { id: 7, name: 'Brainstorm', owner: 1, controller: 1, zone: 'stack', legal: true },
  { id: 8, name: 'Forest', owner: 2, controller: 2, zone: 'library', legal: true },
];
const players = ['You', 'Bob', 'Charlie', 'Diana'].map((name, id) => ({ id, name,
  battlefield: cards.filter(c => c.zone === 'battlefield' && c.controller === id),
  ...Object.fromEntries(['hand','graveyard','exile','command','library'].map(zone => [`${zone}_cards`, cards.filter(c => c.zone === zone && c.owner === id)])),
}));
const game = { objectDetails: async id => cards.find(card => String(card.id) === String(id)) };
function Preview() {
  const [kind, setKind] = useState(new URLSearchParams(location.search).get('kind') || 'select_objects');
  const [result, setResult] = useState('Hover a card to inspect its zone and owner.');
  const base = { player: 0, min: 1, max: 1, description: 'Choose a card', source_id: 99 };
  const decision = kind === 'select_options' ? { ...base, kind, options: cards.map((card, index) => ({ index, description: `Choose ${card.name}`, object_id: card.id, legal: true })) }
    : kind === 'targets' ? { ...base, kind, requirements: [{description:'Target a card', min_targets:1, max_targets:1, legal_targets: cards.map(card => ({kind:'object', object:card.id, name:card.name}))}] }
    : { ...base, kind, candidates: cards };
  const state = { viewed_cards: { cards: cards.filter(c => c.zone === 'library'), card_ids: [8] }, players, perspective: 0, decision, stack_objects: cards.filter(c => c.zone === 'stack'), stack:[7] };
  const Component = { select_objects: SelectObjectsDecision, select_options: SelectOptionsDecision, targets: TargetsDecision }[kind];
  return <I18nProvider><GameContext.Provider value={{game, state, playerAccentOverrides:{}, dispatch: command => setResult(JSON.stringify(command))}}><HoverProvider key={kind}><DragProvider><CombatArrowProvider><ObjectSelectionProvider><TooltipProvider>
    <main className="zone-preview">
      <div className="eyebrow">IRONSMITH / DECISION MENU</div>
      <h1>Every card has a place.</h1>
      <p className="intro">Zone icons identify the owner. A different controller colors the card’s name.</p>
      <div className="legend">{players.map((p,i)=><span key={p.id} style={{color:['#b79cff','#ff3b30','#22c55e','#f28c28'][i]}}><i />{p.name}</span>)}</div>
      <div className="preview-grid">
        <section data-decision-menu className="menu"><div className="menu-title">Choose a card <span>1 selection</span></div>
          <Component key={kind} decision={decision} canAct layout="strip" hideDescription inlineSubmit />
        </section>
        <aside><div className="eyebrow">OWNER ≠ CONTROLLER</div><h2>Rhystic Study</h2><p>The violet battlefield icon says you own this card.</p><p>The red name says Bob controls it.</p><div className="note">Hover the icon for the full zone, owner, and controller description.</div>
          <label>Preview menu<select value={kind} onChange={e=>setKind(e.target.value)}><option value="select_objects">Object selection</option><option value="select_options">Resolution options</option><option value="targets">Targets</option></select></label>
        </aside>
      </div><output>{result}</output>
    </main><FloatingCardPreview />
  </TooltipProvider></ObjectSelectionProvider></CombatArrowProvider></DragProvider></HoverProvider></GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Preview />);
const style = document.createElement('style');
style.textContent = `body {margin:0;background:#151413;color:#ece3d4;font-family:Inter,system-ui,sans-serif;} .zone-preview{max-width:930px;margin:0 auto;padding:45px 30px;} .eyebrow{font-size:10px;letter-spacing:2px;color:#a99a80;font-weight:700;} h1{font-size:34px;letter-spacing:-1px;margin:10px 0;} .intro{color:#afa79b;font-size:14px;} .legend{display:flex;gap:25px;margin:24px 0 28px;font-size:12px}.legend span{display:flex;align-items:center;gap:7px}.legend i{width:7px;height:7px;border-radius:50%;background:currentColor}.preview-grid{display:grid;grid-template-columns:minmax(0,1.7fr) minmax(0,1fr);gap:32px}.menu{background:#211d19;border:1px solid #514635;padding:16px;min-width:0}.menu-title{font-size:14px;font-weight:650;border-bottom:1px solid #423a30;padding-bottom:14px;margin-bottom:10px}.menu-title>span{float:right;font-size:11px;color:#a99a80;font-weight:400}.menu .decision-option-row{min-height:40px}.menu .decision-option-label{font-size:13px}.menu .decision-strip-options-row{flex-direction:column!important;align-items:stretch!important}.menu .decision-strip-scroll{max-height:none!important}.menu .decision-option-row--strip{max-width:none;width:100%}aside{padding-top:16px}h2{font-size:21px;margin:12px 0;color:#ff3b30}aside p,.note{font-size:13px;line-height:1.7;color:#c0b6a6}.note{border-top:1px solid #40382f;padding-top:16px;margin-top:20px;color:#928b81}label{display:block;margin-top:22px;font-size:11px;color:#a99a80}select{display:block;width:100%;background:#26211c;border:1px solid #514635;padding:8px;color:#e5d9c5;margin-top:7px}output{display:block;font-size:11px;color:#958c7e;margin-top:20px}@media(max-width:600px){.zone-preview{padding:24px 16px}h1{font-size:27px}.preview-grid{grid-template-columns:1fr}.legend{gap:16px}aside{padding-top:0}}`;
document.head.appendChild(style);
