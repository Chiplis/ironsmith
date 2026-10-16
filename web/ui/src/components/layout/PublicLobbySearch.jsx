import useUiText from "@/i18n/useUiText";
import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { PUBLIC_FORMATS, relayBaseUrl } from '@/lib/relay/formats';

export default function PublicLobbySearch({ onSelect }) {
  const ui = useUiText();
  const [lobbies, setLobbies] = useState([]);
  const [query, setQuery] = useState('');
  const [format, setFormat] = useState('');
  const [error, setError] = useState('');
  const [loaded, setLoaded] = useState(false);
  const [page, setPage] = useState(0);
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    if (!relayBaseUrl()) return;
    const controller = new AbortController();
    let timer;
    async function load() {
      try {
        if (!document.hidden) {
          const response = await fetch(`${relayBaseUrl()}/lobbies`, { signal: AbortSignal.any([controller.signal, AbortSignal.timeout(10000)]), cache: 'no-store' });
          if (!response.ok) throw new Error('Lobby search is unavailable. Try refreshing.');
          const result = await response.json();
          if (!Array.isArray(result.lobbies)) throw new Error('Invalid lobby directory');
          if (controller.signal.aborted) return;
          setLobbies(result.lobbies); setLoaded(true); setError('');
        }
      } catch (e) {
        if (controller.signal.aborted) return;
        setLobbies([]); setError(e.message);
      }
      timer = setTimeout(load, 30000);
    }
    load();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [refresh]);
  const matches = lobbies.filter(l => (!format || l.format === format)
    && `${l.name} ${l.format}`.toLowerCase().includes(query.toLowerCase()));
  const pageCount = Math.max(1, Math.ceil(matches.length / 2));
  const currentPage = Math.min(page, pageCount - 1);
  return <section className="fantasy-sheet-section grid gap-3 p-4" aria-label={ui("Public WebSocket lobbies")}>
    <div className="flex items-center justify-between gap-2"><strong>{ui("Public lobbies")}</strong>
      <Button variant="secondary" onClick={() => setRefresh(n => n + 1)}>{ui("Refresh")}</Button></div>
    <p className="text-sm text-muted-foreground">{ui("Find a table by format. These lobbies use open decklists (Trusted mode).")}</p>
    <input className="fantasy-field px-3 py-2" aria-label={ui("Search public lobbies")} placeholder={ui("Search host or format")} value={query} onChange={e => { setQuery(e.target.value); setPage(0); }} />
    <select className="fantasy-field px-3 py-2" aria-label={ui("Filter public lobbies by format")} value={format} onChange={e => { setFormat(e.target.value); setPage(0); }}>
      <option value="">{ui("All formats")}</option>{Object.values(PUBLIC_FORMATS).map(f => <option key={f.id} value={f.id}>{ui(f.label)}</option>)}
    </select>
    <div role="status" className="text-sm text-muted-foreground">{error || (!loaded ? ui('Searching for lobbies…') : matches.length ? ui("{0} available {1}", { 0: matches.length, 1: matches.length === 1 ? 'lobby' : 'lobbies' }) : ui('No available lobbies. Create one to advertise your table.'))}</div>
    {matches.slice(currentPage * 2, currentPage * 2 + 2).map(lobby => <Button key={lobby.id} variant="secondary" className="h-auto justify-between whitespace-normal py-3 text-left" onClick={() => onSelect(lobby.id)}>
      <span>{lobby.name} · {PUBLIC_FORMATS[lobby.format]?.label || lobby.format}</span>
      <span>{lobby.playerCount}/{lobby.desiredPlayers}{" " + ui("· Select")}</span>
    </Button>)}
    {pageCount > 1 && <div className="flex items-center justify-between gap-2">
      <button type="button" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>{ui("Previous")}</button>
      <span>{currentPage + 1} / {pageCount}</span>
      <button type="button" disabled={currentPage + 1 >= pageCount} onClick={() => setPage(currentPage + 1)}>{ui("Next")}</button>
    </div>}
  </section>;
}
