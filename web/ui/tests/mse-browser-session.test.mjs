import test from 'node:test';
import assert from 'node:assert/strict';
import {createBrowserMseSession} from '../src/lib/mse/browser-session.mjs';

function fixture(){
  const paths=['data/example.mse-style/style','data/example.mse-game/game','Magic - Fonts/A Font.ttf','Magic - Fonts/Broken.otf'];
  const manifest={sha:'fixture',tree:paths.map(path=>({type:'blob',path}))};
  const calls=[],fonts={disposed:false,loaded:[],async registerFile(bytes){if(bytes[0]===2)throw new Error('Broken font');this.loaded.push(bytes);},dispose(){this.disposed=true;}};
  const fetcher=async(url,options)=>{
    calls.push({url,options});
    return {ok:true,json:async()=>manifest,arrayBuffer:async()=>new Uint8Array([url.includes('Broken')?2:1]),
      text:async()=>url.endsWith('/style')?'game: example\ncard width: 10\ncard height: 12\n':'card field:\n\tname: name\n\ttype: text\n'};
  };
  return {manifest,calls,fonts,options:{corpusUrl:'https://assets.example/mse/',expectedRevision:'fixture',fetcher,createFonts:()=>fonts}};
}
test('browser session discovers styles, loads fonts and shares one owned renderer',async()=>{
  const f=fixture(),session=await createBrowserMseSession(f.options);
  assert.deepEqual(session.styles,['example.mse-style']);
  assert.equal(session.renderer.fieldUpdateMode,'ordered');
  assert.equal(f.fonts.loaded.length,1);assert.equal(session.fontFailures.length,1);
  assert.ok(f.calls.some(c=>c.url.endsWith('A%20Font.ttf')));
  const template=await session.renderer.template('example.mse-style');
  assert.equal(await session.renderer.template('example.mse-style'),template);
  assert.equal(session.store.exists('/example.mse-style/style'),true);
  session.dispose();session.dispose();
  assert.equal(f.fonts.disposed,true);assert.ok(f.calls.every(c=>c.options.signal.aborted));
  await assert.rejects(session.renderer.template('example.mse-style'),{code:'renderer-disposed'});
});
test('browser session rejects an incomplete or unsafe corpus before loading fonts',async()=>{
  for(const mutate of [f=>f.manifest.truncated=true,f=>f.manifest.sha='wrong',f=>f.manifest.tree.push({type:'blob',path:'data/../outside'})]){
    const f=fixture();mutate(f);
    await assert.rejects(createBrowserMseSession(f.options),{code:'corpus-index'});
    assert.equal(f.calls.length,1);assert.equal(f.fonts.loaded.length,0);
  }
});
test('cancellation during font registration disposes resources and cannot return a live session',async()=>{
  const f=fixture(),controller=new AbortController();let entered,release;
  const ready=new Promise(r=>{entered=r;}),gate=new Promise(r=>{release=r;});
  f.fonts.registerFile=async()=>{entered();await gate;};
  const pending=createBrowserMseSession({...f.options,signal:controller.signal});
  const rejected=assert.rejects(pending,{code:'render-aborted'});
  await ready;controller.abort();release();await rejected;
  assert.equal(f.fonts.disposed,true);
});
