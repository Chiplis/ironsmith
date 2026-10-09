import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import ForgeBoard from '../src/components/board/ForgeBoard.jsx';
import { createForgeMeasurer } from '../src/components/board/forge/measure-forge.js';
import { useEffect } from 'react';
export function Fixture() {
  const [scenario, setScenario] = useState({ counts: [3, 7], hand: 7 });
  const [clicks, setClicks] = useState(0);
  useEffect(() => {
    window.forgeScenario = setScenario;
    window.forgeMeasure = createForgeMeasurer(document.querySelector('[data-workspace-shell]'));
  }, []);
  return <main data-workspace-shell style={{ position: 'relative', width: '100vw', height: '100vh', overflow: 'hidden', color: '#e5d9c5', fontFamily: 'Georgia' }}>
    {!scenario.unmounted && <ForgeBoard playerAccentOverrides={scenario.accents} state={{ players: scenario.counts.map((_, id) => ({ id })), perspective: scenario.counts.length - 1, turn_number: scenario.turn || 1, active_player: 0, phase: scenario.combat ? 'Combat' : 'Main' }} interactionLocked={scenario.locked} />}
    <div style={{ position: 'absolute', inset: '35px 65px 130px', display: 'grid', gridTemplateRows: `repeat(${scenario.counts.length}, minmax(0,1fr))`, gap: 25 }}>
      {scenario.counts.map((count, player) => <section key={player} data-arena-owner={String(player)} style={{ minHeight: 0, position: 'relative' }}>
        {scenario.piles && <div style={{ position: 'absolute', right: -58, top: 25 }}>
          {['graveyard', 'exile'].map(zone => <div className="zone-pile-slot" key={zone} style={{ width: 40, height: 48 }}><button data-zone-pile={zone} data-zone-owner={String(player)}>{zone}</button></div>)}
        </div>}
        <header className="battlefield-panel-header" style={{ height: 26 }}>Player {player + 1} · {count} permanents</header>
        <div className="battlefield-row" style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignContent: 'start', height: 'calc(100% - 26px)', overflow: 'auto' }}>
          {Array.from({ length: count }, (_, i) => <button key={i} className="game-card battlefield-row-card" onClick={() => setClicks(n => n + 1)} style={{ flex: '0 0 65px', height: 91, border: '1px solid #9d8459', borderRadius: 5, color: '#d9c79f', background: 'linear-gradient(145deg,#3d514b,#182623)', boxShadow: '0 3px 6px #0008', position: 'relative' }}>{i % 3 ? 'Creature' : 'Land'} {i + 1}{scenario.attachments && i === 0 && <span className="game-card" style={{ position: 'absolute', top: 60, left: 40, width: 65, height: 80, background: '#66513b' }}>Aura</span>}</button>)}
        </div>
      </section>)}
    </div>
    <div className="hand-zone-surface" style={{ position: 'absolute', bottom: 12, left: '15%', width: '70%', height: 95, display: 'flex', overflow: 'auto', gap: 3 }}>{Array.from({ length: scenario.hand || 0 }, (_, i) => <div className="game-card" key={i} style={{ flex: '0 0 60px', height: 85, background: '#313945', border: '1px solid #716851' }}>Hand {i + 1}</div>)}</div>
    {scenario.viewer && <aside className="zone-viewer" data-zone-id="graveyard" style={{ position: 'absolute', inset: '10px auto auto 0', width: 260, height: 240, background: '#202b34' }}>Graveyard / Exile</aside>}
    <output style={{ position: 'absolute', top: 4, right: 15 }}>{clicks}</output>
  </main>;
}
createRoot(document.getElementById('root')).render(<Fixture />);
