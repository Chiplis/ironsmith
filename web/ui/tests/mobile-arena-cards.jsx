import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import { useScryfallImage } from '../src/hooks/useScryfallImageUrl';
import MobileArenaCardFace from '../src/components/cards/MobileArenaCardFace';
import '../src/index.css';
import '../src/components/cards/battlefield-appearance.css';
import '../src/components/board/forge/forge-board.css';
const samples = [
  {name:'Command Tower',type_line:'Land',produced_mana:['W','U','B','R','G']},
  {name:'Wastes',type_line:'Basic Land',produced_mana:['C']},
  {name:'Misty Rainforest',type_line:'Land',produced_mana:[]},
  {name:'Llanowar Elves',type_line:'Creature — Elf Druid',power_toughness:'1/1',colors:['G']},
  {name:'Forest',type_line:'Basic Land — Forest',colors:['G']},
  {name:'Liliana of the Veil',type_line:'Legendary Planeswalker — Liliana',loyalty:3,colors:['B']},
  {name:'Invasion of Tarkir',type_line:'Battle — Siege',defense:5,colors:['R']},
  {name:'Sol Ring',type_line:'Artifact'},
  {name:'Omniscience',type_line:'Enchantment',colors:['U']},
  {name:'The Eldest Reborn',type_line:'Enchantment — Saga',colors:['B'],oracle_text:'I — Each opponent sacrifices a creature.\nII — Each opponent discards a card.\nIII — Return a card.'},
  {name:'Llanowar Elves',type_line:'Creature — Elf Druid',power_toughness:'1/1',token:true,colors:['G']},
  {name:'Hallowed Fountain',type_line:'Land — Plains Island',tapped:true,colors:['W','U'],count:2},
];
function Sample({card}) {
  const {url,ready} = useScryfallImage(card.name,'art_crop');
  const stat = card.power_toughness ?? card.loyalty ?? card.defense;
  const land = /Land/.test(card.type_line);
  return <figure style={{margin:0,width:110}}><div className={`game-card field-card battlefield-arena-card battlefield-artwork-card ${card.tapped?'tapped':''}`}
    style={{position:'relative',width:land?70:92,height:land?58:80,minHeight:0,'--arena-card-height':land?'44px':'80px'}}>
    {card.count && <div className="battlefield-group-stack"><div className="battlefield-group-stack-layer" style={{'--group-stack-depth':1}} /></div>}
    <div className="game-card-surface"><MobileArenaCardFace card={card} name={card.name} artUrl={url} pending={!ready} primary={stat!=null?{label:stat,title:card.type_line}:null}/></div>
    </div><figcaption style={{fontSize:10}}>{card.type_line}{card.token?' token':''}</figcaption></figure>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><main className="mobile-mtga-scene" style={{height:'100dvh',padding:14,display:'flex',flexWrap:'wrap',gap:12,alignContent:'start'}}>{samples.map((card,i)=><Sample key={i} card={card}/>)}</main></I18nProvider>);
