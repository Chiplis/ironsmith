/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import CommunityFormatEditor from '../src/components/layout/CommunityFormatEditor';
import { GameContext } from '../src/context/GameContext.shared';
import { emptyCommunityFormat } from '../src/lib/community-format';
import '../src/index.css';
function Fixture() {
  const [kind, setKind] = useState('format');
  const [set, setSet] = useState(emptyCommunityFormat);
  const [format, setFormat] = useState(emptyCommunityFormat);
  return <main className="fantasy-sheet p-6" style={{ maxWidth: 800, margin: 'auto' }}><h1>Build library</h1><button onClick={() => setKind('set')}>Sets</button><button onClick={() => setKind('format')}>Formats</button>
    {kind === 'set' ? <CommunityFormatEditor kind="set" value={set} onChange={setSet} /> : <CommunityFormatEditor value={format} onChange={value => { setFormat(value); window.__format = value; }} />}
  </main>;
}
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{ game: { autocompleteCardNames: async () => [] } }}><Fixture /></GameContext.Provider></I18nProvider>);
