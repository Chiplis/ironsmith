import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import LobbyDeckCardList from '../src/components/layout/LobbyDeckCardList';
import '../src/index.css';

const params = new URLSearchParams(window.location.search);
const deckText = params.has('empty') ? '' : [
  '4 Lightning Bolt', '4 Lightning Bolt', '2 Mental Misstep', '20 Mountain', '',
  'Sideboard', '3 Pyroblast', '1 Mental Misstep',
].join('\n');
createRoot(document.getElementById('root')).render(<I18nProvider>
  <LobbyDeckCardList deckText={deckText} commanderText={params.has('commander') ? '1 Krenko, Mob Boss' : ''}
    format="modern" cardProblems={{ 'Mental Misstep': 'Banned in Modern' }} />
</I18nProvider>);
