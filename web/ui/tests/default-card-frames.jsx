import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import MiniatureCardFrame from '../src/components/cards/MiniatureCardFrame';
import catalog from '../src/lib/default-card-frames.generated.json';
import '../src/index.css';
const art = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300"><rect width="400" height="300" fill="#243950"/><circle cx="200" cy="150" r="95" fill="#d3b477"/><path d="M130 180L200 60L270 180Z" fill="#476749"/></svg>')}`;
const sagaArt = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="200" height="500"><rect width="200" height="500" fill="#243950"/><rect width="200" height="20" fill="#ff0000"/><rect y="480" width="200" height="20" fill="#00ff00"/><circle cx="100" cy="250" r="75" fill="#d3b477"/></svg>')}`;
const entries = [...Object.keys(catalog).map((key, index) => ({ key, card: {
  id: index + 1, name: `Default ${key}`, colors: ['C', 'land', 'colorless'].includes(key) ? [] : key.split(''),
  layout: 'normal', type_line: key === 'land' ? 'Land' : key === 'C' ? 'Artifact Creature — Construct' : 'Creature — Test',
  mana_cost: key === 'land' ? '' : ['C', 'colorless'].includes(key) ? '{3}' : [...key].map(c => `{${c}}`).join(''),
  oracle_text: 'Vigilance\nWhenever this creature attacks, draw a card.', power: '2', toughness: '3', power_toughness: key === 'land' ? undefined : '2/3', controller: 0,
} })), { key: 'saga', card: { id: 99, name: 'Custom Saga', layout: 'saga', type_line: 'Enchantment — Saga', colors: ['G'], oracle_text: 'I — Draw a card.' } }, { key: 'urzas-saga', card: { id: 100, name: "Urza's Saga", type_line: 'Enchantment Land — Urza’s Saga', color_identity: [], oracle_text: 'I — This Saga gains “{T}: Add {C}.”\nII — Create a Construct.\nIII — Search your library.' } }];
export default function DefaultFrames() {
  return <I18nProvider>{entries.map(({ key, card }) => <section key={key} data-template={key}>
    <p style={{ color: 'white', font: '14px sans-serif' }}>{key}: {catalog[key]?.donor.name || 'Identity-matched Saga frame'}</p>
    <div style={{ position: 'relative', width: 244, height: 340 }}><MiniatureCardFrame card={card} imageUrl={key.includes('saga') ? sagaArt : art} /></div>
  </section>)}</I18nProvider>;
}
createRoot(document.getElementById('frames')).render(<DefaultFrames />);
