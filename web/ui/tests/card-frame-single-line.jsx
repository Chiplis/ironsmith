import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import CardFrameSingleLine from '../src/components/right-rail/CardFrameSingleLine';
import { ManaCostIcons } from '../src/lib/mana-symbols';
import '../src/index.css';
import '../src/styles/card-typography.css';

function Fixture() {
  const [long, setLong] = useState(true);
  const [mana, setMana] = useState(true);
  const [large, setLarge] = useState(false);
  return <>
    <button onClick={() => setLong(!long)}>Change text</button>
    <button onClick={() => setMana(!mana)}>Toggle mana</button>
    <button onClick={() => setLarge(!large)}>Change preferred size</button>
    <div id="panel-host" style={{ width: 420, height: 600 }}>
      <div className="interactive-card-frame-stage" data-card-era="modern" style={{
        height: '100%', '--card-title-font': 'Matrix', '--card-title-weight': 700,
        '--card-type-font': 'MPlantin', '--card-type-weight': 400,
        '--sampled-title-font-size': large ? '22px' : '18px',
        '--sampled-type-font-size': large ? '18px' : '14px',
      }}>
        <article className="interactive-card-frame">
          <div className="interactive-card-frame__inner">
            <header className="interactive-card-frame__title-row">
              <div className="interactive-card-frame__title-wrap">
                <span className="interactive-card-frame__count">×12</span>
                <CardFrameSingleLine as="h2" className="interactive-card-frame__title">
                  {long ? 'Asmoranomardicadaistinaculdacar' : 'Myr'}
                </CardFrameSingleLine>
              </div>
              {mana && <div className="interactive-card-frame__mana"><ManaCostIcons cost="{2}{W}{U}{B}{R}{G}" size={18}/></div>}
            </header>
            <div className="interactive-card-frame__art"><div className="interactive-card-frame__art-fallback"/></div>
            <div className="interactive-card-frame__type-row">
              <CardFrameSingleLine className="interactive-card-frame__type">
                {long ? 'Legendary Artifact Creature — Phyrexian Human Artificer' : 'Artifact'}
              </CardFrameSingleLine>
            </div>
          </div>
        </article>
      </div>
    </div>
  </>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
