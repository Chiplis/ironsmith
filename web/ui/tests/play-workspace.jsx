/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import LobbyOverlay from '../src/components/layout/LobbyOverlay';
import '../src/index.css';
const value = {
  state: { players: [{ id: 0, name: 'Alice' }, { id: 1, name: 'Bob' }], perspective: 0 },
  multiplayer: { mode: 'idle', players: [] }, playerAccentOverrides: {},
  setStatus: () => {}, status: '', game: null,
};
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={value}><LobbyOverlay onClose={() => {}} onStartGoldfish={() => {}} initialCreateDeckText={'4 Lightning Bolt\n4 Monastery Swiftspear\n4 Goblin Guide\n4 Lava Spike\n4 Rift Bolt\n4 Skewer the Critics\n4 Searing Blaze\n4 Boros Charm\n4 Skullcrack\n20 Mountain\n4 Sacred Foundry'} /></GameContext.Provider></I18nProvider>);
