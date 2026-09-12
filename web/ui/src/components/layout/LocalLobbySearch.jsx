import useUiText from "@/i18n/useUiText";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";

export default function LocalLobbySearch({ onSelect }) {
  const ui = useUiText();
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
    <section className="fantasy-sheet-section grid gap-3 p-4" aria-label={ui("Local lobbies")}>
      <div className="text-sm font-semibold">{ui("Local network lobbies")}</div>
      <p className="text-sm text-muted-foreground">{ui("Open this same game address on every device. Available lobbies appear here automatically.")}</p>
      <input
        className="fantasy-field w-full px-3 py-2 text-sm"
        aria-label={ui("Search local lobbies")}
        placeholder={ui("Search host or format")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      <div role="status" className="text-sm text-muted-foreground">
        {error || (!loaded ? ui("Searching for lobbies…") : matches.length === 0 ? ui("No available lobbies found.") : ui("{0} available {1}", { 0: matches.length, 1: matches.length === 1 ? "lobby" : "lobbies" }))}
      </div>
      {matches.map((lobby) => (
        <Button key={lobby.id} variant="secondary" className="h-auto justify-between whitespace-normal py-3 text-left" onClick={() => onSelect(lobby.id)}>
          <span>{lobby.name} · {lobby.format} · {ui(lobby.securityMode)}</span>
          <span>{lobby.playerCount}/{lobby.desiredPlayers}{" " + ui("· Select")}</span>
        </Button>
      ))}
    </section>
  );
}
