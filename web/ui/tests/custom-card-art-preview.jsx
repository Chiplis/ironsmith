import { useState } from 'react';
import { createRoot } from 'react-dom/client';
import CustomCardArtEditor from '../src/components/layout/CustomCardArtEditor';
import { saveCustomCard, listSavedCustomCards } from '../src/lib/custom-card-library';
import '../src/index.css';

export function Harness() {
  const [face, setFace] = useState({ name: 'Art test', artUrl: '', oracleText: '' });
  const [status, setStatus] = useState('');
  return <main style={{padding: 24, maxWidth: 500}}>
    <CustomCardArtEditor face={face} updateFace={patch => setFace(current => ({ ...current, ...patch }))} />
    {face.artUrl && <img alt="Saved card art" src={face.artUrl} />}
    <button onClick={async () => {
      await saveCustomCard({ layout: 'single', faces: [face] }, {compile: async () => ({ canCreate: true, faces: [] })});
      setStatus('Saved');
    }}>Save card</button>
    <button onClick={async () => { setFace((await listSavedCustomCards())[0].draft.faces[0]); }}>Reopen card</button>
    <p role="status">{status}</p>
  </main>;
}
createRoot(document.getElementById('root')).render(<Harness />);
