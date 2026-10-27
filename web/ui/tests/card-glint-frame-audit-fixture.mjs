const base={name:'Frame audit',type_line:'Creature — Wizard',mana_cost:'{2}{U}',colors:['U'],oracle_text:'Flying',power:'2',toughness:'3'};
export function frameAuditCases(){
  return [
    {id:'ordinary',card:base,kind:'normal'},
    {id:'retro',card:{...base,frame:'1997'},kind:'normal'},
    {id:'eighth',card:{...base,frame:'2003'},kind:'normal'},
    {id:'colorshifted',card:{...base,frame:'2003',frame_effects:['colorshifted']},kind:'normal'},
    {id:'future',card:{...base,frame:'future'},kind:'normal'},
    {id:'class',card:{...base,type_line:'Enchantment — Class',type_line_display:'Encantamiento — Clase',power:undefined,toughness:undefined,oracle_text:'Draw a card.\n{2}{U}: Level 2\nCreatures you control have flying.\n{4}{U}: Level 3\nDraw two cards.'},kind:'class'},
    {id:'leveler',card:{...base,oracle_text:'Level up {1}{U}\nLEVEL 2-4\n3/3\nFlying\nLEVEL 5+\n5/5\nFlying'},kind:'leveler'},
    ...[3,4].map(n=>({id:`walker-${n}`,card:{...base,type_line:'Legendary Planeswalker — Test',power:undefined,toughness:undefined,loyalty:'4',oracle_text:['+1: Draw a card.','0: Scry 1.','−2: Draw two cards.','−7: Draw three cards.'].slice(0,n).join('\n')},kind:n===4?'planeswalker-large':'planeswalker'})),
    {id:'transforming-saga',card:{layout:'transform',card_faces:[{...base,type_line:'Enchantment — Saga',power:undefined,toughness:undefined,oracle_text:'I — Draw a card.\nII — Scry 1.\nIII — Exile this Saga, then return it transformed.'},{...base,name:'Reverse',power:'4',toughness:'5'}]},kind:'saga'},
    {id:'fuse',card:{layout:'split',keywords:['Fuse'],card_faces:[{...base,name:'Left',type_line:'Instant',oracle_text:'Draw a card.',power:undefined,toughness:undefined},{...base,name:'Right',type_line:'Sorcery',oracle_text:'Scry 1.',power:undefined,toughness:undefined}]},kind:'normal'},
  ];
}
