/* eslint-disable react-refresh/only-export-components -- Browser fixture. */
import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import LobbyOverlay from '../src/components/layout/LobbyOverlay';
import '../src/index.css';
const burn = '4 Lightning Bolt\n4 Monastery Swiftspear\n4 Goblin Guide\n4 Lava Spike\n4 Rift Bolt\n4 Skewer the Critics\n4 Searing Blaze\n4 Boros Charm\n4 Skullcrack\n20 Mountain\n4 Sacred Foundry';
// ?active=1 shows a joined Modern lobby whose guest deck breaks the format.
const active = new URLSearchParams(location.search).has('active');
const lobby = {
  mode: 'lobby', role: 'host', lobbyId: 'abc123', localPeerId: 'a', localName: 'Alice', format: 'modern',
  desiredPlayers: 2, startingLife: 20, allowUnreleased: true, matchStarted: false,
  localDeckText: `${burn}\n4 Sol Ring`, localCommanderText: '', localDeckCount: 64, localCommanderCount: 0,
  players: [
    { peerId: 'a', name: 'Alice', index: 0, connected: true, ready: false, deckCount: 64, formatErrors: ['Sol Ring is banned in Modern.'] },
    { peerId: 'b', name: 'Bob', index: 1, connected: true, ready: true, deckCount: 60, formatErrors: [] },
  ],
};
const value = {
  state: { players: [{ id: 0, name: 'Alice' }, { id: 1, name: 'Bob' }], perspective: 0 },
  multiplayer: active ? lobby : { mode: 'idle', players: [] }, playerAccentOverrides: {},
  setStatus: () => {}, status: '', game: null, updateLobbyDeck: () => {}, leaveLobby: () => {},
};
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={value}><LobbyOverlay onClose={() => {}} onStartGoldfish={() => {}} initialCreateDeckText={burn} /></GameContext.Provider></I18nProvider>);
