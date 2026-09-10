import test from 'node:test';
import assert from 'node:assert/strict';
import {translationForFace} from '../src/i18n/cardTranslationFace.js';

test('single-face previews select their own name, type, and rules from old buckets',()=>{
  for(const separator of ['\n//\n','\\n//\\n']) {
    const translation={englishName:'Front // Back',name:['Frente','Dorso'].join(separator),
      typeLine:['Criatura','Tierra'].join(separator),oracleText:['Vuela.','Agrega {G}.'].join(separator)};
    assert.deepEqual(translationForFace(translation,'Back'),{
      englishName:'Back',name:'Dorso',typeLine:'Tierra',oracleText:'Agrega {G}.'});
    assert.equal(translationForFace(translation,'Front').name,'Frente');
    assert.equal(translationForFace(translation,'Front // Back'),translation,'combined card views retain both faces');
    assert.equal(translation.name,['Frente','Dorso'].join(separator),'shared cache entry stays intact');
  }
});

test('missing face fields fall back instead of borrowing another face',()=>{
  assert.equal(translationForFace({englishName:'Front // Back',oracleText:'Only one face'},'Back').oracleText,'');
  const single={englishName:'Omniscience',name:'Omnisciencia'};
  assert.equal(translationForFace(single,'Omniscience'),single);
  assert.equal(translationForFace(null,'Back'),null);
});

test('an alias face with nothing to show is not a translation', async () => {
  const {hasTranslatedFields, translationForFace} = await import('../src/i18n/cardTranslationFace.js');
  const shadow = {englishName: 'Emeritus of Conflict // Lightning Bolt', name: 'Emérita del conflicto', typeLine: '', oracleText: 'Daña primero.'};
  assert.equal(hasTranslatedFields(translationForFace(shadow, 'Lightning Bolt')), false);
  // A legacy bucket that joined both faces still resolves the face it names.
  const joined = {...shadow, name: 'Emérita del conflicto // Relámpago', oracleText: 'Daña primero. // El Relámpago hace 3 puntos de daño a cualquier objetivo.'};
  assert.equal(hasTranslatedFields(translationForFace(joined, 'Lightning Bolt')), true);
  assert.equal(translationForFace(joined, 'Lightning Bolt').name, 'Relámpago');
  assert.equal(hasTranslatedFields({englishName: 'Lightning Bolt', name: 'Relámpago', typeLine: '', oracleText: ''}), true);
  assert.equal(hasTranslatedFields(null), false);
});
