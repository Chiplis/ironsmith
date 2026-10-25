import test from 'node:test';
import assert from 'node:assert/strict';
import { build } from 'vite';
import { resolve } from 'node:path';
import { chromium } from 'playwright';

async function bundle(path) {
  const result=await build({configFile:false,logLevel:'silent',build:{write:false,minify:false,lib:{entry:resolve(path),formats:['es']}}});
  return (Array.isArray(result)?result[0]:result).output[0].code;
}
async function load(page,source,name) {
  await page.evaluate(async ({source,name})=>{
    const url=URL.createObjectURL(new Blob([source],{type:'text/javascript'}));
    window[name]=await import(url); URL.revokeObjectURL(url);
  },{source,name});
}

test('native choreography holds omitted properties, sequences steps and releases feedback', async()=>{
  const source=await bundle('src/lib/motion/native-effects.js');
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage(); await page.setContent('<div id="card" style="width:100px;height:140px;transform:translateX(20px)"></div>');
    await page.evaluate(()=>{window.jsFrames=0;const raf=window.requestAnimationFrame;window.requestAnimationFrame=(...args)=>{window.jsFrames++;return raf(...args);};});
    await load(page,source,'motion');
    const result=await page.evaluate(async()=>{
      const card=document.getElementById('card');
      const timeline=window.motion.createEffectTimeline();
      timeline.add(card,{keyframes:[{translateY:0,opacity:0,duration:0},{opacity:1,duration:100},{translateY:100,duration:100}],ease:'linear'});
      timeline.add(card,{translateY:[100,200],duration:100,ease:'linear'});
      await Promise.resolve();
      const animations=card.getAnimations();
      animations.forEach(a=>{a.pause();a.currentTime=50;});
      const first=new DOMMatrix(getComputedStyle(card).transform);
      const opacity=getComputedStyle(card).opacity;
      animations.forEach(a=>{a.currentTime=250;});
      const second=new DOMMatrix(getComputedStyle(card).transform);
      timeline.cancel(); card.style.transform='translateX(20px)';
      const feedback=window.motion.animateEffect(card,{keyframes:[{'--card-jolt-x':'0px','--card-jolt-scale':1,duration:0},{'--card-jolt-x':'10px','--card-jolt-scale':1.2,duration:100}],ease:'linear'});
      feedback.pause(); feedback.currentTime=50;
      const during={translate:getComputedStyle(card).translate,scale:getComputedStyle(card).scale,transform:getComputedStyle(card).transform};
      feedback.finish(); await new Promise(resolve=>setTimeout(resolve,50));
      const outward=window.motion.animateEffect(card,{x:80,duration:100,ease:'linear'});
      outward.finish(); await new Promise(resolve=>setTimeout(resolve,30));
      const returning=window.motion.animateEffect(card,{x:0,duration:100,ease:'linear'});
      returning.pause(); returning.currentTime=50;
      const returningX=new DOMMatrix(getComputedStyle(card).transform).m41;
      returning.finish(); await new Promise(resolve=>setTimeout(resolve,30));
      const restingX=new DOMMatrix(getComputedStyle(card).transform).m41;
      return {first:[first.m41,first.m42],opacity,second:[second.m41,second.m42],during,returningX,restingX,after:getComputedStyle(card).translate,active:card.getAnimations().length,jsFrames:window.jsFrames};
    });
    assert.deepEqual(result.first,[20,0]); assert.equal(result.opacity,'0.5');
    assert.deepEqual(result.second,[20,150]);
    assert.equal(result.during.translate,'5px');
    assert.match(result.during.transform,/matrix\(1\.1, 0, 0, 1\.1, 20, 0\)/);
    assert.equal(result.after,'none'); assert.equal(result.active,0);
    assert.equal(result.returningX,60); assert.equal(result.restingX,20);
    assert.equal(result.jsFrames,0,'DOM animation playback schedules no JavaScript frames');
    await page.emulateMedia({reducedMotion:'reduce'});
    const reduced=await page.evaluate(async()=>{
      const card=document.getElementById('card');let completed=0;
      window.motion.animateEffect(card,{opacity:[0,1],duration:5000,loop:true,onComplete:()=>completed++});
      await new Promise(resolve=>setTimeout(resolve,80));
      return {completed,active:card.getAnimations().length};
    });
    assert.deepEqual(reduced,{completed:1,active:0});
  } finally {await browser.close();}
});

test('WebGL particles move without per-frame uploads and effect promises survive context loss',async()=>{
  const renderer=await bundle('src/lib/motion/stack-fx-renderer.js');
  const effects=await bundle('src/lib/motion/stack-fx.js');
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:800,height:600}});
    await page.setContent('<style>body{background:#101723}.stack-fx-canvas{position:fixed;inset:0;pointer-events:none}</style><canvas id="gpu"></canvas>');
    await load(page,renderer,'gpu'); await load(page,effects,'fx');
    const result=await page.evaluate(async()=>{
      const canvas=document.getElementById('gpu');
      const renderer=new window.gpu.StackFxRenderer(canvas); renderer.resize(800,600,1);
      const gl=renderer.gl;
      let uploads=0; const upload=gl.bufferSubData.bind(gl); gl.bufferSubData=(...args)=>{uploads++;upload(...args);};
      renderer.emit({x:100,y:100,vx:4,vy:0,life:1000,size:8,alpha:1,rgb:'255, 100, 50'});
      const read=()=>{
        const pixels=new Uint8Array(800*600*4); gl.readPixels(0,0,800,600,gl.RGBA,gl.UNSIGNED_BYTE,pixels);
        let weight=0,x=0;
        for(let i=0;i<pixels.length;i+=4){weight+=pixels[i];x+=((i/4)%800)*pixels[i];}
        return {weight,x:x/weight};
      };
      const now=performance.now(); renderer.begin();renderer.render(now);const first=read();
      renderer.render(now+160);const later=read();
      const error=gl.getError(); renderer.dispose();canvas.remove();
      const started=performance.now();
      const point=await window.fx.launchComet({from:{x:100,y:100},to:()=>({x:300,y:180}),duration:100});
      window.fx.burst({at:point,kind:'shards'});window.fx.shockwave({at:point});
      const pending=window.fx.sigil({rect:{left:100,top:100,width:100,height:140},duration:5000});
      const surface=document.querySelector('.stack-fx-canvas');
      const context=surface.getContext('webgl2');
      context.getExtension('WEBGL_lose_context').loseContext();
      await pending;
      return {uploads,first,later,error,point,elapsed:performance.now()-started,hidden:surface.style.display};
    });
    assert.equal(result.uploads,1);assert.equal(result.error,0);
    assert.ok(result.first.weight>0);assert.ok(result.later.x>result.first.x+15);
    assert.deepEqual(result.point,{x:300,y:180});assert.ok(result.elapsed<2500);
    assert.equal(result.hidden,'none');
  } finally {await browser.close();}
});

test('unavailable WebGL skips visual effects without hanging gameplay',async()=>{
  const source=await bundle('src/lib/motion/stack-fx.js');
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage();await page.setContent('<body></body>');
    await page.evaluate(()=>{HTMLCanvasElement.prototype.getContext=()=>null;});
    await load(page,source,'fx');
    const result=await page.evaluate(async()=>{
      await window.fx.launchComet({from:{x:0,y:0},to:{x:100,y:100}});
      await window.fx.sigil({rect:{left:0,top:0,width:100,height:100}});
      await window.fx.chargePulse({rect:{left:0,top:0,width:100,height:100}});
      return document.querySelectorAll('canvas').length;
    });
    assert.equal(result,0);
  } finally {await browser.close();}
});
