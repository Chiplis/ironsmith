import React from 'react';
import { createRoot } from 'react-dom/client';
import { I18nProvider } from '../src/i18n/I18nContext';
import MiniatureCardFrame from '../src/components/cards/MiniatureCardFrame';
import catalog from '../src/lib/default-card-frames.generated.json';
import '../src/index.css';
const art = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300"><rect width="400" height="300" fill="#243950"/><circle cx="200" cy="150" r="95" fill="#d3b477"/><path d="M130 180L200 60L270 180Z" fill="#476749"/></svg>')}`;
const entries = [...Object.keys(catalog).map((key, index) => ({ key, card: {
  id: index + 1, name: `Default ${key}`, colors: ['C', 'land', 'colorless'].includes(key) ? [] : key.split(''),
  layout: 'normal', type_line: key === 'land' ? 'Land' : key === 'C' ? 'Artifact Creature — Construct' : 'Creature — Test',
  mana_cost: key === 'land' ? '' : ['C', 'colorless'].includes(key) ? '{3}' : [...key].map(c => `{${c}}`).join(''),
  oracle_text: 'Vigilance\nWhenever this creature attacks, draw a card.', power: '2', toughness: '3', power_toughness: key === 'land' ? undefined : '2/3', controller: 0,
} })), { key: 'saga', card: { id: 99, name: 'Custom Saga', layout: 'saga', type_line: 'Enchantment — Saga', colors: ['G'], oracle_text: 'I — Draw a card.' } }];
export default function DefaultFrames() {
  return <I18nProvider>{entries.map(({ key, card }) => <section key={key} data-template={key}>
    <p style={{ color: 'white', font: '14px sans-serif' }}>{key}: {catalog[key]?.donor.name || 'Existing Saga fallback'}</p>
    <div style={{ position: 'relative', width: 244, height: 340 }}><MiniatureCardFrame card={card} imageUrl={art} /></div>
  </section>)}</I18nProvider>;
}
createRoot(document.getElementById('frames')).render(<DefaultFrames />);
