import { useEffect, useState } from 'react';
import { listSavedDeckPresets, SAVED_DECKS_CHANGED } from '@/lib/saved-decks';

export default function useSavedDecks() {
  const [decks, setDecks] = useState([]);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    let revision = 0;
    const refresh = async () => {
      const current = ++revision;
      try {
        const entries = await listSavedDeckPresets();
        if (active && current === revision) { setDecks(entries); setError(null); }
      } catch (error) {
        if (active && current === revision) setError(error);
      } finally {
        if (active && current === revision) setLoading(false);
      }
    };
    const channel = typeof BroadcastChannel !== 'undefined' ? new BroadcastChannel(SAVED_DECKS_CHANGED) : null;
    if (channel) channel.onmessage = refresh;
    window.addEventListener(SAVED_DECKS_CHANGED, refresh);
    refresh();
    return () => { active = false; channel?.close(); window.removeEventListener(SAVED_DECKS_CHANGED, refresh); };
  }, []);
  return { decks, setDecks, error, loading };
}
