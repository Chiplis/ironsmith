import { setupCombatPriorityFixture } from './fixtures/combat-priority-scenario.mjs';
import { useEffect, useMemo, useRef } from 'react';
import { createRoot } from 'react-dom/client';
import { GameContext } from '../src/context/GameContext.shared';
import { HoverProvider } from '../src/context/HoverContext';
import { DragProvider } from '../src/context/DragContext';
import { I18nProvider } from '../src/i18n/I18nContext';
import HandZone from '../src/components/board/HandZone';
import { useGameSnapshot } from '../src/hooks/useGameSnapshot';
import { createSnapshotDecoder } from '../src/lib/snapshot-channel';
import { subscribePriorityAnalysisSnapshots } from '../src/lib/priority-analysis-scheduler';
import { buildMultiplayerSmartAutoPass } from '../src/lib/priority-automation';
import '../src/index.css';

function Fixture() {
  const { state, setState, stateRef, subscribeState } = useGameSnapshot();
  const latest = useRef(null), listeners = useRef(new Set());
  const game = useMemo(() => ({ latestPriorityAnalysis: () => latest.current,
    subscribePriorityAnalysis: fn => { listeners.current.add(fn); return () => listeners.current.delete(fn); } }), []);
  const send = useRef(null), passed = useRef(false);
  useEffect(() => subscribePriorityAnalysisSnapshots({ game, getState: () => stateRef.current, setState, subscribeState }), [game, setState, stateRef, subscribeState]);
  useEffect(() => {
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    const decoder = createSnapshotDecoder(), requests = new Map();
    let id = 0;
    const call = (method, args = []) => new Promise((resolve, reject) => {
      requests.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
    });
    send.current = call;
    worker.onmessage = async ({ data }) => {
      if (data.type === 'ready') {
        await call('registerExternalCardSourcesJson', [JSON.stringify(window.__combatFixture.sources)]);
        setState(await setupCombatPriorityFixture((method, ...args) => call(method, args), window.__combatFixture.seat));
      } else if (data.type === 'priorityAnalysis') {
        latest.current = data; for (const listener of listeners.current) listener();
      } else if (data.type === 'priorityAnalysisError') window.__combatError = data.error;
      else if (data.type === 'result') {
        const entry = requests.get(data.id); if (!entry) return;
        requests.delete(data.id);
        if (!data.ok) { window.__combatError = data.error; entry.reject(new Error(data.error.message)); }
        else entry.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
      }
    };
    worker.postMessage({ type: 'init', assetBaseUrl: location.origin + '/' });
    return () => worker.terminate();
  }, [setState]);
  useEffect(() => {
    window.__combatState = state;
    const auto = buildMultiplayerSmartAutoPass({ autoPassEnabled: true, holdRule: 'if_actions', decision: state?.decision, currentState: state });
    if (auto.command && !passed.current) {
      passed.current = true; window.__combatAutoPassed = true;
      send.current('dispatch', [auto.command]).then(setState);
    }
  }, [state, setState]);
  const player = state?.players?.find(p => p.id === state.perspective);
  return <I18nProvider><GameContext.Provider value={{ state, multiplayer: { matchStarted: true, submittingAction: false } }}>
    <HoverProvider><DragProvider><main style={{ height: '100vh', background: '#14171c', display: 'flex', alignItems: 'flex-end' }}>
      {player && <HandZone player={player} isExpanded layout="fan" />}
    </main></DragProvider></HoverProvider>
  </GameContext.Provider></I18nProvider>;
}
createRoot(document.getElementById('root')).render(<Fixture />);
