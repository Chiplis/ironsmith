import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import HoverArtOverlay from '../src/components/right-rail/HoverArtOverlay';
import '../src/index.css';
const card = { zone: new URLSearchParams(window.location.search).get('zone') || 'Battlefield', id: 1, name: 'Ornithopter', type_line: 'Artifact Creature — Thopter', mana_cost: '{0}', oracle_text: 'Flying', power: 0, toughness: 2 };
const state = { players: [{ id: 0, battlefield: [card] }], perspective: 0 };
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{ state }}>
  <div id="card-host" style={{ position: 'relative', width: 420, height: 670, margin: 30 }}>
    <HoverArtOverlay objectId={1} displayMode="card-frame" transientPreview={{card}} />
  </div>
</GameContext.Provider></I18nProvider>);
