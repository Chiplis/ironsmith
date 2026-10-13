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

test('font decoration metrics retain signed positions and design-unit thicknesses',()=>{
  const tables=[['name',fixture().subarray(28)],['head',new Uint8Array(20)],['post',new Uint8Array(12)],['OS/2',new Uint8Array(30)],['hhea',new Uint8Array(10)]];
  const head=new DataView(tables[1][1].buffer),post=new DataView(tables[2][1].buffer),os2=new DataView(tables[3][1].buffer);
  head.setUint16(18,2048);post.setInt16(8,-200);post.setInt16(10,80);os2.setInt16(26,90);os2.setInt16(28,600);
  const hhea=new DataView(tables[4][1].buffer);hhea.setInt16(4,1500);hhea.setInt16(6,-400);hhea.setInt16(8,200);
  const bytes=new Uint8Array(92+tables.reduce((n,[,t])=>n+t.length,0)),view=new DataView(bytes.buffer);
  view.setUint32(0,0x10000);view.setUint16(4,5);let offset=92;
  tables.forEach(([tag,data],i)=>{const at=12+i*16;bytes.set([...tag].map(c=>c.charCodeAt(0)),at);view.setUint32(at+8,offset);view.setUint32(at+12,data.length);bytes.set(data,offset);offset+=data.length;});
  assert.deepEqual(readFontMetadata(bytes).verticalMetrics,{unitsPerEm:2048,ascent:1500,descent:-400,lineGap:200});
  assert.deepEqual(readFontMetadata(bytes).decoration,{unitsPerEm:2048,underlinePosition:-200,underlineThickness:80,strikeThickness:90,strikePosition:600});
});
