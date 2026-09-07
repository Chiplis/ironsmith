import test from 'node:test';
import assert from 'node:assert/strict';
import {locateManaSymbols,manaTemplateSource} from '../src/lib/card-mana-match.js';

for (const vertical of [false,true]) test(`registers ${vertical?'vertical off-title':'horizontal title'} mana with reversed ink`,()=>{
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4).fill(190);
  const icons=[0,1].map(key=>{
    const pixels=new Uint8ClampedArray(24*24*4);
    for(let y=0;y<24;y++)for(let x=0;x<24;x++) {
      const ink=((Math.floor(x/4)*17+Math.floor(y/4)*31+key*13)%11)<5;
      pixels.set([ink?20:220,ink?20:220,ink?20:220,255],(y*24+x)*4);
    }
    return {key:String(key),data:pixels,width:24,height:24};
  });
  const origin=vertical?{x:70,y:94}:{x:394,y:38};
  icons.forEach((icon,i)=>{
    for(let y=0;y<24;y++)for(let x=0;x<24;x++) {
      const at=((origin.y+y+(vertical?i*29:0))*width+origin.x+x+(vertical?-i*14:i*29))*4;
      const value=255-icon.data[(y*24+x)*4];data.set([value,value,value,255],at);
    }
  });
  const result=locateManaSymbols({data,width,height},{x:30,y:30,width:430,height:40},icons,
    vertical?{vertical:true,bounds:{x:50,y:85,width:65,height:85}}:{});
  assert.ok(result);
  for(const [i,symbol] of result.symbols.entries()) {
    assert.ok(Math.abs(symbol.x-origin.x-(vertical?-i*14:i*29))<=2);
    assert.ok(Math.abs(symbol.y-origin.y-(vertical?i*29:0))<=2);
    assert.ok(Math.abs(symbol.width-24)<=2);
  }
});

test('rejects mana registration on a flat panel',()=>{
  const scan={width:488,height:680,data:new Uint8ClampedArray(488*680*4).fill(200)};
  const icon={key:'G',width:24,height:24,data:new Uint8ClampedArray(24*24*4)};
  assert.equal(locateManaSymbols(scan,{y:30,height:40},[icon]),null);
});

test('mask templates cover Phyrexian and hybrid costs used by the visible renderer',()=>{
  for(const color of ['W','U','B','R','G']) {
    assert.equal(manaTemplateSource(`${color}/P`),`https://svgs.scryfall.io/card-symbols/${color}P.svg`);
    assert.equal(manaTemplateSource(`2/${color}`),`https://svgs.scryfall.io/card-symbols/2${color}.svg`);
  }
  assert.equal(manaTemplateSource('W/U'),'https://svgs.scryfall.io/card-symbols/WU.svg');
  assert.equal(manaTemplateSource('G/U/P'),'https://svgs.scryfall.io/card-symbols/GUP.svg');
  assert.ok(manaTemplateSource('R').startsWith('data:image/svg+xml,'));
  assert.equal(manaTemplateSource('invalid'),null);
});
