import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,mergeMechanics} from '../../web/ui/src/lib/custom-mechanics.js';
const conflict=/could not be added|conflicting/i;
test('explicit collective, price, chosen-number and permanent-link policies cannot silently replace a known definition',()=>{
 const cases=[
  [{keyword:'Archive weave',template:'{Object}',reminder:'Exile {Object}.',actionProgram:true,collectiveObject:true},{collectiveObject:false}],
  [{keyword:'Price echo',template:'{Object}',reminder:'Exile {Object}.',actionProgram:true,operationPaymentAlternative:{defaultPhrase:'exile a card from your hand'}},{operationPaymentAlternative:{defaultPhrase:'sacrifice a creature'}}],
  [{keyword:'Countbound',template:'{Cost}',reminder:'{Cost}: Choose a number for this permanent.',numberedCharacteristics:true},{numberedCharacteristics:false}],
  [{keyword:'Match weave',reminder:'This creature gets +1/+1.',permanentLinkRelation:'matching bonds'},{permanentLinkRelation:'other bonds'}],
 ];
 for(const [raw,changed]of cases){const known=validateMechanic(raw);assert.throws(()=>mergeMechanics([known],[{...raw,...changed}]),conflict);}
});
test('omitted inline policy metadata retains the explicit source definition and normalized price phrases agree',()=>{
 const raw={keyword:'Countbound',template:'{Cost}',reminder:'{Cost}: Choose a number for this permanent.',numberedCharacteristics:true};
 const {numberedCharacteristics,...inline}=raw;
 assert.equal(mergeMechanics([raw],[inline])[0].numberedCharacteristics,true);
 const price={keyword:'Price echo',template:'{Object}',reminder:'Exile {Object}.',actionProgram:true,operationPaymentAlternative:{defaultPhrase:'exile a card from your hand'}};
 assert.doesNotThrow(()=>mergeMechanics([price],[{...price,operationPaymentAlternative:{defaultPhrase:'Exile  a card from your hand'}}]));
 assert.throws(()=>validateMechanic({...raw,numberedCharacteristics:'true'}),/boolean/);
 assert.throws(()=>validateMechanic({keyword:'Match weave',reminder:'This creature gets +1/+1.',permanentLinkRelation:['matching bonds']}),/plain source-defined name/);
});
