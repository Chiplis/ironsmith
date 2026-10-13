import test from 'node:test';
import assert from 'node:assert/strict';
import { readFontMetadata } from '../src/lib/mse/font-metadata.mjs';
function fixture() {
  const records=[[1,'Example'],[2,'Bold Italic'],[16,'Example Extended']];
  const storage=6+records.length*12,length=storage+records.reduce((n,[,s])=>n+s.length*2,0);
  const bytes=new Uint8Array(28+length),view=new DataView(bytes.buffer);
  view.setUint32(0,0x10000);view.setUint16(4,1);bytes.set([110,97,109,101],12);
  view.setUint32(20,28);view.setUint32(24,length);view.setUint16(30,records.length);view.setUint16(32,storage);
  let cursor=0;
  records.forEach(([id,name],i)=>{
    const at=34+i*12;view.setUint16(at,3);view.setUint16(at+2,1);view.setUint16(at+4,0x409);view.setUint16(at+6,id);view.setUint16(at+8,name.length*2);view.setUint16(at+10,cursor);
    for(const c of name){view.setUint16(28+storage+cursor,c.charCodeAt(0));cursor+=2;}
  });
  return bytes;
}
test('font names use table offsets, UTF-16, and both family naming schemes',()=>{
  const bytes=fixture(),wrapped=new Uint8Array(bytes.length+10);wrapped.set(bytes,5);
  const info=readFontMetadata(wrapped.subarray(5,5+bytes.length));
  assert.deepEqual(info.families,['Example','Example Extended']);
  assert.equal(info.subfamily,'Bold Italic');
  assert.equal(info.weight,400);
});
test('font metadata rejects truncated tables and invalid formats',()=>{
  assert.throws(()=>readFontMetadata(new Uint8Array(5)),{code:'font-format'});
  const bytes=fixture();new DataView(bytes.buffer).setUint32(20,bytes.length);
  assert.throws(()=>readFontMetadata(bytes),{code:'font-format'});
  const unknown=fixture();unknown[0]=42;
  assert.throws(()=>readFontMetadata(unknown),{code:'font-format'});
});
