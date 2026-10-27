export function specialLayoutCard(layout) {
  const front={name:'Guardian',type_line:'Creature — Human',mana_cost:'{2}{G}',oracle_text:'Vigilance',power:'3',toughness:'4'};
  const back={name:'Journey',type_line:'Sorcery',mana_cost:'{1}{U}',oracle_text:'Draw a card.'};
  if(layout==='room') {
    front.type_line=back.type_line='Enchantment — Room';
    delete front.power;delete front.toughness;
  }
  if(layout==='aftermath') {
    front.type_line='Sorcery';front.oracle_text='Draw a card.';
    delete front.power;delete front.toughness;
    back.oracle_text='Aftermath\nDraw a card.';
  }
  if(layout==='flip') Object.assign(back,{type_line:'Legendary Creature — Spirit',power:'5',toughness:'6',oracle_text:'Flying'});
  if(layout==='battle') {
    Object.assign(front,{type_line:'Battle — Siege',defense:'4',oracle_text:'Draw a card.'});
    delete front.power;delete front.toughness;
    Object.assign(back,{type_line:'Creature — Spirit',power:'5',toughness:'6',oracle_text:'Flying'});
  }
  return {layout,name:'Guardian // Journey',card_faces:[front,back]};
}
