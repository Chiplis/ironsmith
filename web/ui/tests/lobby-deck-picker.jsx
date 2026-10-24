/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React, { useState } from 'react';
import SaveLobbyDeck from '../src/components/layout/SaveLobbyDeck';
import CompetitiveDeckPicker from '../src/components/layout/CompetitiveDeckPicker';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import { LobbyDeckCatalogPicker } from '../src/components/layout/LobbyDeckEditor';
import '../src/index.css';

function ChangingFormatFixture() {
  const [format, setFormat] = useState('modern');
  const [allowUnreleased, setAllowUnreleased] = useState(false);
  return <div data-format-state={format} data-unreleased-state={String(allowUnreleased)}>
    <label>Lobby format<select aria-label="Lobby format" value={format} onChange={event => setFormat(event.target.value)}>
      <option value="modern">Modern</option><option value="pioneer">Pioneer</option>
    </select></label>
    <label><input type="checkbox" checked={allowUnreleased} onChange={event => setAllowUnreleased(event.target.checked)} />Allow unreleased cards</label>
    <LobbyDeckCatalogPicker format={format} allowUnreleased={allowUnreleased} onChange={applied => { window.__applied = applied; }} />
  </div>;
}

function Fixture() {
  // The real picker lives inside the lobby sheet, whose chrome styles every
  // control; the fixture reproduces that scope so the flat rules are tested
  // against it.
  return (
    <div data-slot="sheet-content" style={{ padding: 24, width: 720 }}>
      {new URLSearchParams(window.location.search).has('changing-format') ? <ChangingFormatFixture /> : new URLSearchParams(window.location.search).has('save') ? <>
        <SaveLobbyDeck deckText={'98 Island\nSideboard\n1 Forest'} commanderText={"1 Tymna the Weaver\n1 Kraum, Ludevic's Opus"} format="commander" playerName="Host" />
        <CompetitiveDeckPicker format="commander" onApply={(applied) => { window.__applied = applied; }} />
      </> : <LobbyDeckCatalogPicker format={new URLSearchParams(window.location.search).get('format') || 'modern'} onChange={(applied) => { window.__applied = applied; }} />}
    </div>
  );
}

createRoot(document.getElementById('root')).render(<I18nProvider><Fixture /></I18nProvider>);
