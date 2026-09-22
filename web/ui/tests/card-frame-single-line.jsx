import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import CardFrameSingleLine from '../src/components/right-rail/CardFrameSingleLine';
import { ManaCostIcons } from '../src/lib/mana-symbols';
import '../src/index.css';
import '../src/styles/card-typography.css';
import '../src/styles/card-frame-colors.css';
import '../src/components/right-rail/card-frame-stage.css';

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
        '--card-stats-font': '"Beleren Small Caps"', '--card-stats-weight': 700,
        '--printed-scan-width': 488, '--printed-stats-baseline': 630,
        '--printed-stats-text-bounds': JSON.stringify({x:400,y:610,width:40,height:20}),
        '--printed-pt-left': '87.37%', '--printed-pt-drop': '4px', '--printed-pt-font-size': '19.37px',
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
            <div className="interactive-card-frame__rules-section" data-printed-stats="true" data-pt-treatment="text">
              <div className="interactive-card-frame__art-stats interactive-card-frame__printed-stats">
                <CardFrameSingleLine className="interactive-card-frame__stats-text">{long ? '123/456' : '2/3'}</CardFrameSingleLine>
              </div>
            </div>
          </div>
        </article>
      </div>
    </div>
  </>;
}
createRoot(document.getElementById('root')).render(<Fixture/>);
