import React from 'react';
import { createRoot } from 'react-dom/client';
import { SymbolText, ManaCostIcons } from '../src/lib/mana-symbols.jsx';
const text = 'Agrega {RRR}.';
createRoot(document.getElementById('root')).render(React.createElement('div', null,
  React.createElement('div', { id: 'translated' }, React.createElement(SymbolText, { text, keywordHelpers: false })),
  React.createElement('div', { id: 'canonical' }, React.createElement(SymbolText, { text: 'Agrega {R}{R}{R}.', keywordHelpers: false })),
  React.createElement('div', { id: 'mixed' }, React.createElement(SymbolText, { text: '({T}: Agrega {WUBRG}{CC}.)', keywordHelpers: false })),
  React.createElement('div', { id: 'cost' }, React.createElement(ManaCostIcons, { cost: '{RRR}{12}{W/U}{B/P}{X}' }))
));
