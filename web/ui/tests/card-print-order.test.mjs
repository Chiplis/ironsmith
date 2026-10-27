import test from 'node:test';
import assert from 'node:assert/strict';
import {orderedManaSymbols,printedManaCost,printedTypeLine} from '../src/lib/card-print-order.mjs';
import {cardGlintFace} from '../src/lib/card-glint/card.mjs';
import {cardGlintDraftFace,draftTypeLine} from '../src/lib/card-glint/draft.mjs';
import {setCardPresentation,clearCardPresentations} from '../src/lib/card-glint/printing-choices.mjs';
import {cardFrameSnapshot} from '../src/lib/card-frame-snapshot.mjs';

const permutations = values => values.length < 2 ? [values] : values.flatMap((value,i) =>
  permutations(values.filter((_,j) => i !== j)).map(rest => [value,...rest]));

test('every color combination has the same printed order for every input permutation', () => {
  const orders = ['W','U','B','R','G','WU','UB','BR','RG','GW','WB','UR','BG','RW','GU',
    'WUB','UBR','BRG','RGW','GWU','WBG','URW','BGU','RWB','GUR',
    'WUBR','UBRG','BRGW','RGWU','GWUB','WUBRG'];
  for (const expected of orders) for (const input of permutations([...expected])) {
    assert.deepEqual(orderedManaSymbols(input), [...expected], input.join(''));
  }
});

test('costs retain multiplicity and handle generic, colorless, snow, hybrid and Phyrexian symbols', () => {
  const costs = ['{X}{X}{2}{G}{G}{W}', '{2}{C}{C}{W}', '{1}{S}{S}', '{2/W}{2/U}{2/B}{2/R}{2/G}',
    '{W/B}{U}', '{1}{B/R}{R}{R/G}', '{2}{G}{G/U/P}{U}', '{3}{W}{W/P}',
    '{C/W}{C/U}{C/B}{C/R}{C/G}'];
  for (const cost of costs) {
    const symbols = [...cost.matchAll(/\{([^{}]+)\}/g)].map(match => match[1]);
    assert.equal(printedManaCost(symbols.reverse().map(symbol => `{${symbol}}`).join('')), cost);
  }
  assert.equal(printedManaCost('{W/G}'), '{G/W}');
  assert.equal(printedManaCost('{P/U/G}'), '{G/U/P}');
  assert.equal(printedManaCost('{W/2}'), '{2/W}');
  assert.equal(printedManaCost('Pay {W}{G}.'), 'Pay {W}{G}.');
  assert.equal(printedManaCost('{R} // {W}'), '{R} // {W}');
  assert.equal(printedManaCost(''), '');
});

test('type lines sort recognized main types and preserve subtype and localized text', () => {
  for (const types of permutations(['Legendary','Snow','Enchantment','Artifact','Land','Creature'])) {
    assert.equal(printedTypeLine(types.join(' ')+' — Forest Golem Wizard'),
      'Legendary Snow Enchantment Artifact Land Creature — Forest Golem Wizard');
  }
  assert.equal(printedTypeLine('Artifact Kindred — Wizard Equipment'), 'Kindred Artifact — Wizard Equipment');
  assert.equal(printedTypeLine('Creature Land — Forest Dryad'), 'Land Creature — Forest Dryad');
  assert.equal(printedTypeLine('Creature Artifact // Creature Enchantment'), 'Artifact Creature // Enchantment Creature');
  assert.equal(printedTypeLine('Criatura artefacto — Gólem'), 'Criatura artefacto — Gólem');
  assert.equal(printedTypeLine('Custom Creature — Artifact'), 'Custom Creature — Artifact');
});

test('builder drafts, live snapshots, display overrides and separate faces share ordering without mutating definitions', () => {
  const source = {manaCost:'{W}{2}{G}{G}',supertypes:['Legendary'],cardTypes:['Creature','Artifact'],subtypes:['Golem']};
  const before = JSON.stringify(source);
  assert.equal(draftTypeLine(source), 'Legendary Artifact Creature — Golem');
  const draft = cardGlintDraftFace({layout:'single',faces:[source]});
  assert.deepEqual(draft.manaSymbols, ['2','G','G','W']);
  assert.equal(draft.typeLine, 'Legendary Artifact Creature — Golem');
  assert.equal(JSON.stringify(source),before);
  const card = {stable_id:9001,name:'Builder test',mana_cost:'{W}{G}',type_line:'Creature Artifact — Golem',
    display_mana_cost:'{U}{G}',type_line_display:'Creature Enchantment — Golem',
    card_faces:[{mana_cost:'{W}{R}',type_line:'Creature Land — Forest Dryad'},
      {mana_cost:'{U}{G}{R}',type_line:'Artifact Kindred — Wizard Equipment'}]};
  setCardPresentation(card.stable_id,[card.name],{}, {builderCreated:true});
  const saved = JSON.stringify(card);
  const snapshot = JSON.parse(cardFrameSnapshot(card));
  assert.equal(snapshot.mana_cost, '{G}{W}');
  assert.equal(snapshot.display_mana_cost, '{G}{U}');
  assert.equal(snapshot.type_line_display, 'Enchantment Creature — Golem');
  assert.equal(snapshot.card_faces[0].mana_cost, '{R}{W}');
  assert.equal(snapshot.card_faces[1].mana_cost, '{G}{U}{R}');
  assert.deepEqual(cardGlintFace({...card,display_mana_cost:undefined},{faceIndex:1}).manaSymbols, ['G','U','R']);
  const lightweight = cardGlintFace({card_types:['creature','artifact'],subtypes:['Golem'],mana_cost:'{W}{G}'},{builderCreated:true});
  assert.equal(lightweight.typeLine, 'Artifact Creature — Golem');
  assert.deepEqual(lightweight.manaSymbols, ['G','W']);
  assert.equal(JSON.stringify(card),saved);
  clearCardPresentations();
});


test('baked cards retain supplied ordering even when Build changes their printing', () => {
  const card = {stable_id:9002,name:'Baked card',mana_cost:'{R}{W}{U}',type_line:'Creature Artifact — Golem',
    card_faces:[{mana_cost:'{W}{G}',type_line:'Creature Artifact — Golem'}]};
  for (const printingChoice of [false,true]) {
    if (printingChoice) setCardPresentation(card.stable_id,[card.name],{finish:'prismatic'});
    const snapshot = JSON.parse(cardFrameSnapshot(card));
    assert.equal(snapshot.mana_cost,card.mana_cost);
    assert.equal(snapshot.type_line,card.type_line);
    assert.deepEqual(snapshot.card_faces,card.card_faces);
    assert.equal(cardGlintFace(card).typeLine,'Creature Artifact — Golem');
    assert.deepEqual(cardGlintFace(card).manaSymbols,['W','G']);
  }
  clearCardPresentations();
});
