import React from 'react';
import { createRoot } from 'react-dom/client';
import { SymbolText } from '../src/lib/mana-symbols';
import '../src/index.css';

createRoot(document.getElementById('root')).render(
  <div style={{ padding: 100 }}><SymbolText text={"Flying, cycling {1}{U}, counter target spell.\nMobilize 2\nWard—Pay 3 life.\nSuspend 4—{U}\nMorph {R}\nFabricate 3"} keywordHelpers /></div>
);
