import test from 'node:test';
import assert from 'node:assert/strict';
import {fidelityText} from './card-frame-fidelity-text.js';
const field=(text,...lines)=>({kind:'rule',text,lines:lines.map(text=>({text}))});
test('comparison preserves the printed wording rather than newly added ability words',()=>{
 assert.equal(fidelityText(field('Landfall — Whenever a land you control enters, you gain 1 life.','Whenever a land enters','the battlefield under your control, you gain 1 life.')),'Whenever a land enters the battlefield under your control, you gain 1 life.');
});
test('comparison preserves omitted reminders while recovering the printed mana cost',()=>{
 assert.equal(fidelityText(field('Mutate {5}{U}{U} (If you cast this spell for its mutate cost, ...)', 'Mutate 566')),'Mutate {5}{U}{U}');
 assert.equal(fidelityText(field('{T}: Add {C}.','€: Add one colorless mana to your mana pool.')),'{T}: Add one colorless mana to your mana pool.');
});
test('historical types and unchanged names stay as printed',()=>{
 assert.equal(fidelityText({kind:'type',text:'World Enchantment',lines:[{text:'Enchant World'}]}),'Enchant World');
 assert.equal(fidelityText({kind:'name',text:'Psychic Frog',lines:[]}), 'Psychic Frog');
 assert.equal(fidelityText({kind:'name',text:'Monster',lines:[{text:'MONSTER'}]}), 'MONSTER');
});

test('level-up reminder pips are recovered without changing the printed prose',()=>{
 assert.equal(fidelityText(field('Level up {R}{R} ({R}{R}: Put a level counter on this. Level up only as a sorcery.)','Level up 2 (@: Put a level counter','on this. Level up only as a sorcery.)')),'Level up {R}{R} ({R}{R}: Put a level counter on this. Level up only as a sorcery.)');
});

test('comparison restores spaces between known adjacent words without adding oracle wording',()=>{
 assert.equal(fidelityText(field('Copy target instant or sorcery spell.', 'Copy targetinstant or sorcery spell.')), 'Copy target instant or sorcery spell.');
});

test('inline printed payment pips survive uppercase OCR letter confusion',()=>{
 assert.equal(fidelityText(field("Change the target of target spell with a single target unless that spell’s controller pays {2}.","CHANGE THE TARGET OF TARGET SPELL WITH A SINGLE TARGET UNLESS THAT SPELL’S CONTROLLER PAVS 2.")),"CHANGE THE TARGET OF TARGET SPELL WITH A SINGLE TARGET UNLESS THAT SPELL’S CONTROLLER PAYS {2}.");
});

test('recovering cost pips keeps the printed sacrifice name',()=>{
 assert.equal(fidelityText(field('{1}, {T}, Sacrifice this land: Draw a card.', '1, €, Sacrifice Horizon Canopy: Draw a card.')), '{1}, {T}, Sacrifice Horizon Canopy: Draw a card.');
});

test('mana reconstruction retains the historical sentence following its output',()=>{
 assert.equal(fidelityText(field("{T}: Add {B} or {R}. This land doesn't untap during your next untap step.","€: Add @ or @ to your mana pool. Mogg Hollows doesn't untap during your next untap phase.")),"{T}: Add {B} or {R} to your mana pool. Mogg Hollows doesn't untap during your next untap phase.");
});

test('embedded mana abilities do not replace the preceding historical prose',()=>{
 assert.equal(fidelityText(field('As long as this card is in your graveyard, lands you control have "{T}: Add {G} or {W}."','As long as Riftstone Portal is in your graveyard, lands you control have "@: Add @ or * to your mana pool."')), 'As long as Riftstone Portal is in your graveyard, lands you control have "{T}: Add {G} or {W} to your mana pool."');
 assert.equal(fidelityText(field('({T}: Add {W}, {B}, or {G}.)','(©: Add *, P, or @.)')), '({T}: Add {W}, {B}, or {G}.)');
});

test('planeswalker comparisons recover shield costs without including OCR punctuation in prose',()=>{
 const ability={...field('+1: You gain 2 life. Add {U}, {R}, or {W}. Spend this mana only to cast a noncreature spell.','•: You gain 2 life. Add 2, @, or *. Spend this mana only to cast a noncreature spell.'),loyaltyCost:'+1'};
 assert.equal(fidelityText(ability),'+1: You gain 2 life. Add {U}, {R}, or {W}. Spend this mana only to cast a noncreature spell.');
 assert.equal(fidelityText({...field('−3: Draw a card.','-3: Draw a card.'),inlineLoyalty:true}),'−3: Draw a card.');
});

test('comparison retains printed uppercase type lettering',()=>{
 assert.equal(fidelityText({kind:'type',text:'Legendary Artifact',lines:[{text:'LEGENDARY ARTIFACT'}]}),'LEGENDARY ARTIFACT');
});

test('an inline cost reduction is not mistaken for a keyword cost and truncated',()=>{
 assert.equal(fidelityText(field('Instant and sorcery spells you cast cost {1} less to cast.','Instant and sorcery spells you cast cost 1 less to cast.')),'Instant and sorcery spells you cast cost {1} less to cast.');
});

test('mana output OCR punctuation does not duplicate a historical single-sentence ability',()=>{
 assert.equal(fidelityText(field('{G/W}, {T}: Add {G}{G}, {G}{W}, or {W}{W}.','* *, @: Add * *, *., or ** to your mana pool.')),'{G/W}, {T}: Add {G}{G}, {G}{W}, or {W}{W} to your mana pool.');
 assert.equal(fidelityText(field('At the beginning of your first main phase, add {R}{R}.','At the beginning of your first main phase, add aa.')),'At the beginning of your first main phase, add {R}{R}.');
});

test('joined uppercase words retain their printed capitalization',()=>{
 assert.equal(fidelityText(field('Counter that spell.','COUNTERTHATSPELL.')),'COUNTER THAT SPELL.');
});

test('printed comparisons retain payment qualifiers and repair only source OCR errors',()=>{
 assert.equal(fidelityText(field('Creatures cannot attack you unless their controller pays {2}.','Creatures cannot attack you unless their controller pays an additional 2.')),'Creatures cannot attack you unless their controller pays an additional {2}.');
 assert.equal(fidelityText(field('You gain life.','You gaın lite.')),'You gain life.');
 assert.equal(fidelityText(field('Target creature gets -2/-2 until end of turn.','Target creature gets -2l-2 until end of turn.')),'Target creature gets -2/-2 until end of turn.');
 assert.equal(fidelityText(field('Put a +1/+1 counter on it.','Put a +1l+1 counter on it.')),'Put a +1/+1 counter on it.');
 assert.equal(fidelityText(field('I — Crescent Fang — Search your library.','Crescent Fang Search your library.')),'Crescent Fang — Search your library.');
 assert.equal(fidelityText(field('({T}: Add {B}, {G}, or {U}.)','@(D, or O.): Add)')),'({T}: Add {B}, {G}, or {U}.)');
});


test('printed granted tap symbols recover whitespace-separated OCR glyphs',()=>{
 assert.equal(fidelityText(field('Lands you control have "{T}: Add one mana of any color."', 'Lands you control have "e : Add one mana of any color."')), 'Lands you control have "{T}: Add one mana of any color."');
});
test('level-up reminders survive a missing opening parenthesis in OCR',()=>{
 assert.equal(fidelityText(field('Level up {1}{G} ({1}{G}: Put a level counter on this. Level up only as a sorcery.)','Level up 1 ф','P: Put a level counter','on this. Level up only as a sorcery.)')), 'Level up {1}{G} ({1}{G}: Put a level counter on this. Level up only as a sorcery.)');
});
test('flavor formatting markers are not printed characters',()=>{
 assert.equal(fidelityText({kind:'flavor',text:'—Oracle *en*-Vec'}), '—Oracle en-Vec');
});
