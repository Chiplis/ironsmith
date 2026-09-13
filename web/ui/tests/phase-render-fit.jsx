import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import CardFrameRulesBox from '../src/components/right-rail/CardFrameRulesBox';
import CardFrameSingleLine from '../src/components/right-rail/CardFrameSingleLine';
import '../src/index.css';
function Fixture() {
  const [phase, setPhase] = useState(0);
  const [long, setLong] = useState(true);
  const [large, setLarge] = useState(false);
  return <>
    <button onClick={() => setPhase(n => n + 1)}>Advance phase</button>
    <button onClick={() => setLong(!long)}>Change rules</button>
    <button onClick={() => setLarge(!large)}>Change typography</button>
    <output>{phase}</output>
    {Array.from({length: 24}, (_, i) => <div key={i} className="interactive-card-frame-stage" style={{width:240, height:160, '--sampled-rules-font-size':large?'22px':'18px'}}>
      <CardFrameSingleLine className="interactive-card-frame__title">A very long legendary creature name that needs fitting</CardFrameSingleLine>
      <div style={{height:100,display:'flex'}}><CardFrameRulesBox label="Rules">
        <div className="interactive-card-frame__rules-body"><button disabled={phase % 2 === 0} onClick={() => {}}>
          <span className="interactive-card-frame__rule-line">{long ? 'Whenever another creature enters, draw a card and then discard a card. '.repeat(3) : 'Draw a card.'}</span>
        </button></div>
      </CardFrameRulesBox></div>
    </div>)}
  </>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
