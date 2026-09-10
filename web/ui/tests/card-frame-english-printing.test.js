import test from 'node:test';
import assert from 'node:assert/strict';
import {resolveScryfallEnglishPrinting} from '../src/lib/scryfall.js';

test('English fallback preserves set, collector number, and back face', async () => {
  const originalFetch=globalThis.fetch;
  const printing={lang:'es',set:'abc',collector_number:'12'};
  globalThis.fetch=async url=>{
    assert.equal(url,'https://api.scryfall.com/cards/abc/12/en');
    return {ok:true,json:async()=>({lang:'en',card_faces:[
      {name:'Front',image_uris:{normal:'front'}},
      {name:'Back',image_uris:{normal:'back'}},
    ]})};
  };
  try {
    const result=await resolveScryfallEnglishPrinting('https://cards.scryfall.io/normal/back/a/b/id.jpg',printing);
    assert.equal(result.name,'Back');
    assert.equal(result.image_uris.normal,'back');
    assert.equal(await resolveScryfallEnglishPrinting('front',{...printing,lang:'en'}),null);
  } finally {globalThis.fetch=originalFetch;}
});
