import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput, validateMechanic} from '../src/lib/custom-mechanics.js';
const declaration=keyword=>validateMechanic({keyword,reminder:`{Cost}, Exile this card from your graveyard: Create a token that's a copy of this card. Have those tokens perform the rekindle action. Activate only as a sorcery.`});
test('created-object action declaration binds abbreviated source watcher without rewriting its consequence',()=>{
 const original='Arise {3}{R}{W}\nWhen Bugrund arises, you may put an Equipment card from your hand or graveyard onto the battlefield attached to it.';
 const face=compileDraftInput({faces:[{name:"Bugrund, Firehearth's Last",oracleText:original}]},[declaration('Arise')]).faces[0];
 assert.match(face.oracleText,/Have those tokens perform the rekindle action/);
 assert.match(face.oracleText,/When this permanent performs the rekindle action, you may put an Equipment card from your hand or graveyard onto the battlefield attached to it/);
 assert.equal(face.displayOracleText,original);
});
test('created-object event binding follows an arbitrary declared action keyword',()=>{
 const face=compileDraftInput({faces:[{name:'Witness, Last Keeper',oracleText:'Kindlemark {2}\nWhen Witness kindlemarks, draw a card.'}]},[declaration('Kindlemark')]).faces[0];
 assert.match(face.oracleText,/When this permanent performs the rekindle action, draw a card/);
});
