import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import CardCreationControls from '../src/components/layout/CardCreationControls';
import '../src/index.css';
window.additions = [];
const game = {
  autocompleteCardNames: async query => ['Forest', 'Lightning Bolt'].filter(name => name.toLowerCase().includes(query.toLowerCase())),
  addCardToZone: async (...args) => window.additions.push({ kind: 'supported', args }),
  createCustomCard: async args => window.additions.push({ kind: 'custom', args }),
  previewCustomCard: async () => ({ faces: [], canCreate: false }),
};
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{
  game, state: { perspective: 0, players: [{ id: 0, name: 'Alice' }, { id: 1, name: 'Bob' }] },
  multiplayer: { mode: 'idle', matchStarted: false }, runWasmInteraction: fn => fn(), refresh: async () => {}, setStatus: () => {},
}}><CardCreationControls navigation /></GameContext.Provider></I18nProvider>);
