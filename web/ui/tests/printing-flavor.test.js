import test from 'node:test';
import assert from 'node:assert/strict';
import {localizedPrintingFlavor} from '../src/lib/printing-flavor.js';
test('flavor translation must match the printing, face and requested language',()=>{
  const source={set:'som',collector_number:'82',oracle_id:'oracle',name:'Assault Strobe'};
  const translated={...source,lang:'es',flavor_text:'Para cuando partirle la cara…'};
  assert.equal(localizedPrintingFlavor(source,translated,'es'),translated.flavor_text);
  for(const mismatch of [{lang:'en'},{set:'other'},{collector_number:'83'},{oracle_id:'other'}])assert.equal(localizedPrintingFlavor(source,{...translated,...mismatch},'es'),'');
  const dfc={...translated,card_faces:[{name:'Front',flavor_text:'Frente'},{name:'Back',flavor_text:'Dorso'}]};
  assert.equal(localizedPrintingFlavor({...source,name:'Back'},dfc,'es'),'Dorso');
  assert.equal(localizedPrintingFlavor({...source,name:'Missing'},dfc,'es'),'');
});
