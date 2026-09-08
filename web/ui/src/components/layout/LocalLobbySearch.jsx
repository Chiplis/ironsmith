import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";

export default function LocalLobbySearch({ onSelect }) {
  const [lobbies, setLobbies] = useState([]);
  const [query, setQuery] = useState("");
  const [error, setError] = useState("");
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    const controller = new AbortController();
    let timer;
    async function refresh() {
      try {
        const response = await fetch("/__ironsmith_lan/lobbies", { signal: controller.signal, cache: "no-store" });
        if (!response.ok) throw new Error("Lobby service unavailable");
        const result = await response.json();
        if (!Array.isArray(result.lobbies)) throw new Error("Invalid lobby directory");
        if (controller.signal.aborted) return;
        setLobbies(result.lobbies);
        setError("");
        setLoaded(true);
      } catch {
        if (controller.signal.aborted) return;
        setLobbies([]);
        setError("Cannot reach the local lobby service. Retrying…");
      }
      timer = setTimeout(refresh, 3000);
    }
    refresh();
    return () => { controller.abort(); clearTimeout(timer); };
  }, []);

  const matches = lobbies.filter((lobby) => `${lobby.name} ${lobby.format} ${lobby.id}`.toLowerCase().includes(query.toLowerCase()));
  return (
    <section className="fantasy-sheet-section grid gap-3 p-4" aria-label="Local lobbies">
      <div className="text-sm font-semibold">Local network lobbies</div>
      <p className="text-sm text-muted-foreground">Open this same game address on every device. Available lobbies appear here automatically.</p>
      <input
        className="fantasy-field w-full px-3 py-2 text-sm"
        aria-label="Search local lobbies"
        placeholder="Search host or format"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <div role="status" className="text-sm text-muted-foreground">
        {error || (!loaded ? "Searching for lobbies…" : matches.length === 0 ? "No available lobbies found." : `${matches.length} available ${matches.length === 1 ? "lobby" : "lobbies"}`)}
      </div>
      {matches.map((lobby) => (
        <Button key={lobby.id} variant="secondary" className="h-auto justify-between whitespace-normal py-3 text-left" onClick={() => onSelect(lobby.id)}>
          <span>{lobby.name} · {lobby.format} · {lobby.securityMode}</span>
          <span>{lobby.playerCount}/{lobby.desiredPlayers} · Select</span>
        </Button>
      ))}
    </section>
  );
}
