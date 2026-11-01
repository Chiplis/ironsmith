import React from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import SubtypesPanel from '../src/components/layout/SubtypesPanel';
window.subtypeInputs = [];
const game = { previewCustomCard: async input => {
  window.subtypeInputs.push(input);
  if (input.customSubtypes[0].singular === 'elf') throw new Error('Subtype form is reserved: elf');
  return { canCreate: true };
} };
createRoot(document.getElementById('root')).render(<GameContext.Provider value={{ game }}><SubtypesPanel /></GameContext.Provider>);
