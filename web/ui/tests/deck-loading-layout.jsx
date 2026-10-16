/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import DeckLoadingView from '../src/components/board/DeckLoadingView';
import '../src/index.css';

const players = ['Alice', 'Bob', 'Charlie', 'Diana'].map((name, id) => ({ id, name }));

function Fixture() {
  const value = {
    state: { players, perspective: 0, decision: null },
    multiplayer: { mode: 'idle' },
    playerAccentOverrides: {},
    game: null,
    setStatus: (message) => { window.__status = message; },
  };
  const embedded = new URLSearchParams(location.search).has("embedded");
  return (
    <GameContext.Provider value={value}>
      <div style={{ height: '100vh' }}>
        {embedded ? <div className="play-workspace lobby-sheet lobby-sheet-goldfish" style={{height:'100%',display:'flex',flexDirection:'column'}}>
          <div className="lobby-sheet-body"><div className="lobby-sheet-setup">
            <div className="flex"><button>Goldfish</button></div>
            <DeckLoadingView embedded onTestDecks={() => {}} onCancel={() => {}} />
          </div></div>
        </div> : <DeckLoadingView onTestDecks={() => {}} onCancel={() => {}} />}
      </div>
    </GameContext.Provider>
  );
}

createRoot(document.getElementById('root')).render(<I18nProvider><Fixture /></I18nProvider>);
