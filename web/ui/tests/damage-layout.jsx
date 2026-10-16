import React from 'react';
import {createRoot} from 'react-dom/client';
import {GameContext} from '../src/context/GameContext.shared';
import {I18nProvider} from '../src/i18n/I18nContext';
import GameCard from '../src/components/cards/GameCard';
import '../src/index.css';
const cards=[{id:1,name:'Nicol Bolas, Dragon-God',card_types:['planeswalker'],counters:[{kind:'Loyalty',amount:5}]},{id:2,name:'Grizzly Bears',card_types:['creature'],power_toughness:'4/4',power_toughness_without_counters:'2/2',damage:1,counters:[{kind:'+1/+1',amount:2},{kind:'Shield',amount:1}]},{id:3,name:'Grizzly Bears',card_types:['creature'],power_toughness:'2/2',damage:0}];
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{state:{players:[]},game:null}}><main style={{display:'flex',gap:70,padding:80}}>{cards.map(card=><GameCard key={card.id} card={card} style={{width:150,height:130,'--bf-card-height':'130px','--bf-card-width':'150px'}} />)}</main></GameContext.Provider></I18nProvider>);
