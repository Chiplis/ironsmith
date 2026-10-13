import test from 'node:test';
import assert from 'node:assert/strict';
import {normalizeFont} from '../src/lib/mse/font-normalize.mjs';
function fixture(){
  const b=new Uint8Array(136),v=new DataView(b.buffer);v.setUint32(0,0x10000);v.setUint16(4,3);
  [['head',60,12],['name',72,20],['cmap',92,44]].forEach(([tag,offset,length],i)=>{const at=12+i*16;b.set([...tag].map(c=>c.charCodeAt(0)),at);v.setUint32(at+8,offset);v.setUint32(at+12,length);});
  v.setUint16(74,1);v.setUint16(76,18);v.setUint16(78,3);v.setUint16(80,1);v.setUint16(82,0x409);v.setUint16(84,1);v.setUint16(86,2);v.setUint16(90,65);
  v.setUint16(94,1);v.setUint16(96,3);v.setUint16(98,1);v.setUint32(100,12);
  v.setUint16(104,4);v.setUint16(106,32);v.setUint16(110,4);v.setUint16(112,2);v.setUint16(114,1);
  v.setUint16(118,65);v.setUint16(120,65535);v.setUint16(124,65);v.setUint16(126,65535);
  return b;
}
test('font normalization repairs derived cmap fields and checksums without changing mappings or input',()=>{
  const source=fixture(),before=source.slice(),result=normalizeFont(source);
  assert.deepEqual(source,before);
  assert.deepEqual(result.repairs,['cmap format 4 searchRange']);
  assert.equal(new DataView(result.bytes.buffer).getUint16(112),4);
  assert.deepEqual(result.bytes.slice(118),source.slice(118));
  let sum=0;const v=new DataView(result.bytes.buffer);for(let i=0;i<result.bytes.length;i+=4)sum=(sum+v.getUint32(i))>>>0;
  assert.equal(sum,0xb1b0afba);
  assert.deepEqual(normalizeFont(result.bytes).repairs,[]);
});
