import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { I18nProvider } from '../src/i18n/I18nContext';
import CreateCardForgeSheet from '../src/components/layout/CreateCardForgeSheet';
import '../src/index.css';
const game = {
  previewCustomCard: async draft => {
    for (const face of draft.faces) {
      for (const field of ['loyalty', 'defense']) {
        if (face[field] != null && (!Number.isInteger(face[field]) || face[field] < 0)) {
          throw new Error(`invalid custom card draft: ${field} must be a number or null`);
        }
      }
    }
    return { canCreate: true, faces: [] };
  },
  autocompleteCardNames: async () => [],
  startSandbox: async plan => { window.sandboxPlan = plan; return { created: plan.sandboxer ? [{ id: 999, draft: plan.sandboxer }] : [] }; },
};
createRoot(document.getElementById('root')).render(<I18nProvider><GameContext.Provider value={{
  game, state: { players: [{ id: 0, name: 'Player 1' }, { id: 1, name: 'Player 2' }] }, multiplayer: { mode: 'idle' },
  runWasmInteraction: async fn => fn(), refresh: async () => {}, setStatus: () => {}, semanticThreshold: 0,
  setSemanticThreshold: () => {}, cardsMeetingThreshold: 0,
}}><CreateCardForgeSheet trigger={<button>Build</button>} /></GameContext.Provider></I18nProvider>);
