import test from 'node:test';
import assert from 'node:assert/strict';
import {compressedHandSelection} from '../src/lib/hand-selection-layout.js';

test('selected cards and compressed neighbors stay in the reserved hand footprint', () => {
  for (const count of [1, 2, 7, 15, 30]) {
    const dims = {cardW:150,cardH:210,packWidth:900,hoverScale:2.5};
    const stride = Math.min(120, 700 / Math.max(1,count-1));
    const layout = Array.from({length:count}, (_,i)=>({x:(i-(count-1)/2)*stride,rot:0}));
    for (let active=0;active<count;active++) {
      const result=compressedHandSelection(layout,dims,active);
      result.forEach((card,index)=>{
        const half=dims.cardW*card.scale/2;
        assert.ok(card.x-half>=-450-0.01 && card.x+half<=450+0.01, `${count} cards, selection ${active}, neighbor ${index}`);
        if(index) assert.ok(card.x>=result[index-1].x,'card order stays stable');
      });
      assert.equal(result[active].scale,2.5);
    }
  }
});
