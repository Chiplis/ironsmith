import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import CardFrameStage from '../src/components/right-rail/CardFrameStage';

const stock = { defaultFrame: 'U' }, printed = { defaultFrame: 'prepared' };
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const settle = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolve))));
function Demo() {
  const [renderer, setRenderer] = useState('stock');
  const [preparation, setPreparation] = useState(stock);
  const [identity, setIdentity] = useState('guardian');
  const [enabled, setEnabled] = useState(true);
  const [count, setCount] = useState(0);
  const [results, setResults] = useState('');
  const update = (kind, ready, card = 'guardian') => flushSync(() => {
    setRenderer(kind); setPreparation(ready); setIdentity(card);
  });
  const run = async () => {
    const lines = [];
    const check = (condition, label) => { if (!condition) throw new Error(label); lines.push(`PASS ${label}`); };
    const snapshot = () => document.querySelector('[data-frame-transition-snapshot]');
    try {
      update('stock', stock); await settle(); await sleep(260);
      update('printing', null);
      check(Boolean(snapshot()), 'outgoing renderer remains visible while replacement prepares');
      check(snapshot().getAttribute('aria-hidden') === 'true' && snapshot().hasAttribute('inert'), 'snapshot cannot receive focus or accessibility interaction');
      check(!snapshot().querySelector('[id]'), 'snapshot does not duplicate control IDs');
      await sleep(300);
      check(Boolean(snapshot()), 'slow preparation does not expose a blank frame');
      update('printing', printed); await settle();
      check(snapshot()?.getAnimations().length === 1, 'ready replacement starts a dissolve');
      await sleep(300);
      check(!snapshot(), 'completed transition removes the outgoing DOM');
      update('stock', null); update('printing', null); await settle();
      check(document.querySelectorAll('[data-frame-transition-snapshot]').length === 1, 'rapid renderer switches keep only one snapshot');
      update('stock', stock, 'other-card');
      check(!snapshot(), 'changing card identity discards stale content immediately');
      flushSync(() => setEnabled(false)); update('printing', printed, 'other-card');
      check(!snapshot(), 'disabled animation creates no snapshot');
      flushSync(() => setEnabled(true)); await settle();
      setResults([...lines, 'ALL CHECKS PASSED'].join('\n'));
    } catch (error) { setResults([...lines, `FAIL ${error.message}`].join('\n')); }
  };
  const switchRenderer = async () => {
    const next = renderer === 'stock' ? 'printing' : 'stock';
    update(next, null); await sleep(650); update(next, next === 'stock' ? stock : printed);
  };
  return <main style={{fontFamily:'system-ui', background:'#121719', color:'#ede9df', minHeight:'100vh', padding:32}}>
    <h1>Card renderer dissolve</h1>
    <p>The current frame stays visible until the replacement is ready, then dissolves in place.</p>
    <button onClick={switchRenderer}>Switch renderer</button>{' '}
    <button onClick={run}>Run transition checks</button>{' '}
    <span>Live activations: {count}</span>
    <div style={{position:'relative', width:320, height:446, marginTop:24}}>
      <CardFrameStage animateRenderer={enabled} transitionIdentity={identity} transitionRenderer={renderer}
        preparation={preparation} showLiveFrame className="interactive-card-frame-stage"
        style={{position:'absolute',inset:0,background:renderer==='stock'?'#567986':'#b9c6b7',border:'8px solid #232c30',borderRadius:14,color:'#10191d',boxSizing:'border-box'}}>
        <h2 style={{fontFamily:renderer==='stock'?'system-ui':'Georgia',padding:'0 14px'}}>Library Guardian</h2>
        <img alt="Card artwork" src="/tests/fixtures/bilingual-frames/counterspell-en-art_crop.jpg" style={{width:'100%',height:180,objectFit:'cover'}} />
        <p style={{padding:'0 14px'}}>Creature — Wizard</p>
        <button id="live-card-ability" style={{margin:14}} onClick={() => setCount(value=>value+1)}>Draw a card</button>
        <strong style={{position:'absolute',bottom:16,right:16}}>3/5</strong>
      </CardFrameStage>
    </div>
    <pre aria-label="Transition results">{results}</pre>
  </main>;
}
createRoot(document.getElementById('root')).render(<Demo />);
