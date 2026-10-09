import React, { useCallback, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { useAutomaticHandReveal } from '../src/hooks/useAutomaticHandReveal';
function Fixture() {
  const [blocked, setBlocked] = useState(true);
  const [perspective, setPerspective] = useState(0);
  const [automatic, setAutomatic] = useState(true);
  const [commands, setCommands] = useState([]);
  const [tick, setTick] = useState(0);
  const state = { perspective, snapshot_id: tick, decision: {
    kind: 'select_objects', player: 1, automatic_public_reveal: automatic,
    reveal_policy: 'public', min: 7, max: 7,
    candidates: Array.from({length: 7}, (_, i) => ({id: 10+i, legal: true})),
  } };
  const stateRef = useRef(state); stateRef.current = state;
  const dispatch = useCallback(command => { setCommands(old => [...old, command]); }, []);
  const isBlocked = useCallback(() => blocked, [blocked]);
  useAutomaticHandReveal({state, stateRef, dispatch, isBlocked});
  return <><button onClick={() => setBlocked(false)}>Unblock</button>
    <button onClick={() => setPerspective(1)}>Owner</button>
    <button onClick={() => setTick(n => n+1)}>Rerender</button>
    <button onClick={() => setAutomatic(false)}>Manual</button>
    <button onClick={() => setAutomatic(true)}>Automatic</button>
    <output>{JSON.stringify(commands)}</output></>;
}
createRoot(document.getElementById('root')).render(<React.StrictMode><Fixture /></React.StrictMode>);
