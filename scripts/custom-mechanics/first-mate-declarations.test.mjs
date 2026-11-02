import test from 'node:test';
import assert from 'node:assert/strict';
import { compileDraftInput, validateMechanic } from '../../web/ui/src/lib/custom-mechanics.js';
const reminder = 'If this card outside the game is your chosen first mate, you may swap it with a card in your opening hand.';
const condition = 'Each nonland card in your starting deck has “+1/+1 counter” or “harmonize” in its printed rules text.';
const definition = validateMechanic({ keyword: 'First mate', reminder });
test('first mate reads its declaration criterion without changing original presentation', () => {
 const source = `Type: Creature\nFirst mate — ${condition} *(${reminder})*`;
 const draft = { faces: [{ name: 'Lilia of the Soul', oracleText: source }] };
 const result = compileDraftInput(draft, [definition]).faces[0];
 assert.match(result.oracleText, /Chosen outside game card "first mate": Each nonland card/);
 assert.match(result.oracleText, /"\+1\/\+1 counter" or "harmonize" in its printed rules text/);
 assert.equal(draft.faces[0].oracleText, source);
 assert.match(result.displayOracleText, /First mate —/);
});
test('same action reminder supports another designation and claims only a complete bounded declaration', () => {
 const other = validateMechanic({keyword:'Deck ally',reminder:reminder.replace('first mate','deck ally')});
 const input = {faces:[{name:'Another choice',oracleText:'Deck ally — Each nonland card in your starting deck has "ward" in its printed rules text.'}]};
 assert.match(compileDraftInput(input,[other]).faces[0].oracleText,/Chosen outside game card "deck ally"/);
 const quoted = {faces:[{name:'Quoted choice',oracleText:'Creatures have "First mate — '+condition+'".'}]};
 assert.doesNotMatch(compileDraftInput(quoted,[definition]).faces[0].oracleText,/Chosen outside game card/);
});
