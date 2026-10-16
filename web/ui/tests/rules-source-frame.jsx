import React from 'react';
import {createRoot} from 'react-dom/client';
import {I18nProvider} from '../src/i18n/I18nContext';
import CardFramePresentation from '../src/components/cards/CardFramePresentation';
import '../src/index.css';
const sources = [
  ['initiative','The Initiative','Venture into Undercity.'],
  ['monarch','The Monarch','Draw a card.'],
  ['ring','The Ring','Each opponent loses 3 life.'],
  ['dungeon_room','Secret Entrance','Search your library for a basic land card, reveal it, put it into your hand, then shuffle.'],
];
createRoot(document.getElementById('root')).render(<I18nProvider><main style={{display:'flex',gap:20,padding:20,background:'#10151c'}}>
  {sources.map(([source_kind,name,ability_text])=><div key={source_kind} style={{width:280,height:392}}><CardFramePresentation card={{source_kind,name,ability_text}}/></div>)}
</main></I18nProvider>);
