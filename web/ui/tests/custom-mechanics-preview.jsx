import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { SymbolText } from '../src/lib/mana-symbols';
import { saveCustomCard } from '../src/lib/custom-card-library.js';
import { variukDraft } from './custom-mechanics-fixture.mjs';
import '../src/index.css';

export function Harness() {
  const [status, setStatus] = useState('Ready');
  const [text, setText] = useState('');
  async function save(kind) {
    setStatus('Saving');
    const draft = variukDraft();
    if (kind === 'broken') draft.faces[0].oracleText += '\nThis line intentionally cannot compile.';
    if (kind === 'conflicting') draft.faces[0].oracleText = draft.faces[0].oracleText.replace('cost 1 less', 'cost 2 less');
    try {
      const saved = await saveCustomCard(draft, {compile: input => window.mechanicTestGame.previewCustomCard(input)});
      const { compileDraftInput } = await import('../src/lib/custom-mechanics.js');
      const preview = window.mechanicTestGame.previewCustomCard(compileDraftInput(saved.draft, saved.draft.customMechanics));
      setText(preview.faces[0].compiledText.join('\n'));
      setStatus(`Saved ${saved.draft.faces[0].name}`);
    } catch (error) { setStatus(`Save failed: ${String(error.message || error)}`); }
  }
  return <main style={{padding: 80, maxWidth: 700}}>
    <button onClick={() => save('broken')}>Save broken card</button>{' '}
    <button onClick={() => save('conflicting')}>Save conflicting card</button>{' '}
    <button onClick={() => save('valid')}>Save Variuk</button>
    <p role="status">{status}</p>
    <div data-testid="compiled-card"><SymbolText text={text} keywordHelpers /></div>
  </main>;
}
createRoot(document.getElementById('root')).render(<Harness />);
