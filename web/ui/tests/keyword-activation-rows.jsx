import {createRoot} from 'react-dom/client';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import {I18nProvider} from '../src/i18n/I18nContext';
import {interactiveRulesView} from '../src/lib/inspector-ability-lines';
import {registerCustomCardAssets} from '../src/lib/custom-card-assets';
import '../src/index.css';
const improve={keyword:'improve',template:'{Cost}',reminder:'{Cost}: Put a +1/+1 counter on this permanent. This ability costs {1} more to activate for each +1/+1 counter on this permanent. Activate only as a sorcery.'};
const advance={keyword:'advance',reminder:"You may pay {Cost} rather than pay this spell's mana cost.\nAs this permanent enters, if its advance cost was paid, have this permanent perform the advance action.\n{Cost}: Have this permanent perform the advance action. Activate only as a sorcery."};
const examples=[
 {id:'improve',name:'Herald of Tedium',type_line:'Creature — Human Warlock',mana_cost:'{1}{B}',power:'2',toughness:'1',oracle_text:'Improve {B}\nWhenever you put one or more +1/+1 counters on this creature, reveal the top card of your library and put it into your hand. You lose life equal to its mana value.',definitions:[improve],activation:'{B}: Put a +1/+1 counter on this permanent. Activate only as a sorcery.'},
 {id:'advance',name:'Shearing Vampire',type_line:'Artifact Creature — Vampire Rogue',mana_cost:'{2}{B}',power:'2',toughness:'2',oracle_text:'Advance {4}{B}\nWhenever Shearing Vampire advances, target player discards a card.',definitions:[advance],activation:'{4}{B}: Have this permanent perform the advance action. Activate only as a sorcery.'},
 {id:'compiled',name:'Herald of Tedium (compiled)',type_line:'Creature — Human Horror Warlock',mana_cost:'{1}{B}',power:'3',toughness:'2',oracle_text:'Improve.\n{B}: Put a +1/+1 counter on this creature. Activate only as a sorcery.\nThis ability costs {1} more to activate for each +1/+1 counter on this permanent.\nWhenever you put one or more +1/+1 counters on this creature, reveal the top card of your library and put it into your hand. You lose life equal to its mana value.',definitions:[improve],activation:'{B}: Put counters.'},
 {id:'equip',name:'Variable Equipment',type_line:'Artifact — Equipment',mana_cost:'{1}',oracle_text:'Equipped creature gets +1/+1.\nEquip {X}',definitions:[],activation:'{X}: Attach this Equipment to target creature you control. Activate only as a sorcery.'},
];
for(const card of examples)registerCustomCardAssets([{draft:{attribution:{artist:'Test fixture'},faces:[{name:card.name}],customMechanics:card.definitions}}]);
window.rowActivations=[];
createRoot(document.getElementById('root')).render(<I18nProvider><main style={{display:'flex',flexWrap:'wrap',gap:40,padding:20}}>{examples.map(card=><section key={card.id} data-example={card.id} style={{width:488,height:680,position:"relative"}}><CardFramePresentation card={card} rulesView={interactiveRulesView(card.oracle_text,card.oracle_text,[{id:card.id,kind:'activate_ability',label:`Activate ${card.name}: ${card.activation}`,available:true}],card.definitions)} onActivate={action=>window.rowActivations.push(action.id)}/></section>)}</main></I18nProvider>);
