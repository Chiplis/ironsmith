import { useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Badge } from "@/components/ui/badge";
import { LOBBY_DECK_SIZE, parseDeckList } from "@/lib/decklists";

const pill =
  "text-[13px] uppercase cursor-pointer hover:brightness-125 transition-all select-none";
const inputClass =
  "w-full rounded-md border border-[#344a61] bg-[#0b1118] px-3 py-2 text-[14px] text-foreground outline-none focus:border-primary/60";
const labelClass =
  "grid gap-1 text-[12px] uppercase tracking-[0.18em] text-muted-foreground";
const textareaClass =
  "min-h-[220px] w-full rounded-md border border-[#344a61] bg-[#0b1118] p-3 text-[14px] text-foreground outline-none focus:border-primary/60 font-mono resize-none";

function formatPlayerStatus(player, localPeerId) {
  if (player.connected === false) return "Offline";
  if (player.ready) return player.peerId === localPeerId ? "You / Ready" : "Ready";
  const deckCount = Number(player.deckCount || 0);
  return player.peerId === localPeerId
    ? `You / ${deckCount}/${LOBBY_DECK_SIZE}`
    : `${deckCount}/${LOBBY_DECK_SIZE}`;
}

export default function LobbyOverlay({
  onClose,
  defaultName = "Player",
  defaultStartingLife = 20,
}) {
  const {
    multiplayer,
    createLobby,
    joinLobby,
    leaveLobby,
    updateLobbyDeck,
  } = useGame();
  const [mode, setMode] = useState("create");
  const [createName, setCreateName] = useState(defaultName);
  const [joinName, setJoinName] = useState(defaultName);
  const [joinCode, setJoinCode] = useState("");
  const [desiredPlayers, setDesiredPlayers] = useState(2);
  const [startingLife, setStartingLife] = useState(defaultStartingLife);
  const [createDeckText, setCreateDeckText] = useState("");
  const [joinDeckText, setJoinDeckText] = useState("");

  const lobbyActive = multiplayer.mode !== "idle";
  const playerCount = multiplayer.players.length;
  const readyPlayers = multiplayer.players.filter((player) => player.ready).length;
  const slotsRemaining = Math.max(0, multiplayer.desiredPlayers - playerCount);
  const createDeckCount = useMemo(
    () => parseDeckList(createDeckText).length,
    [createDeckText]
  );
  const joinDeckCount = useMemo(
    () => parseDeckList(joinDeckText).length,
    [joinDeckText]
  );
  const localPlayer = multiplayer.players.find(
    (player) => player.peerId === multiplayer.localPeerId
  );
  const localReady = Boolean(localPlayer?.ready);
  const startPending =
    !multiplayer.matchStarted &&
    multiplayer.mode === "starting";

  const handleCreate = () => {
    createLobby({
      name: createName,
      desiredPlayers,
      startingLife,
      deckText: createDeckText,
    });
  };

  const handleJoin = () => {
    joinLobby({
      name: joinName,
      lobbyId: joinCode,
      deckText: joinDeckText,
    });
  };

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-[#04070dcc]/85 px-4">
      <div className="w-full max-w-4xl rounded-xl border border-[#2b3e55] bg-[linear-gradient(180deg,#101826_0%,#0a121d_100%)] p-4 shadow-[0_24px_80px_rgba(0,0,0,0.45)]">
        <div className="mb-4 flex items-start justify-between gap-3">
          <div className="grid gap-1">
            <span className="text-[11px] uppercase tracking-[0.24em] text-[#7d97b4]">
              Multiplayer
            </span>
            <h2 className="text-[24px] font-bold uppercase tracking-[0.16em] text-foreground">
              Create Lobby
            </h2>
          </div>
          <Badge variant="secondary" className={pill} onClick={onClose}>
            Close
          </Badge>
        </div>

        {!lobbyActive ? (
          <div className="grid gap-4">
            <div className="flex gap-2">
              <Badge
                variant="secondary"
                className={`${pill} ${
                  mode === "create" ? "brightness-125" : "opacity-70"
                }`}
                onClick={() => setMode("create")}
              >
                Create
              </Badge>
              <Badge
                variant="secondary"
                className={`${pill} ${
                  mode === "join" ? "brightness-125" : "opacity-70"
                }`}
                onClick={() => setMode("join")}
              >
                Join
              </Badge>
            </div>

            {mode === "create" ? (
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px]">
                <div className="grid gap-4">
                  <label className={labelClass}>
                    Your Name
                    <input
                      className={inputClass}
                      value={createName}
                      onChange={(event) => setCreateName(event.target.value)}
                      placeholder="Host name"
                    />
                  </label>
                  <label className={labelClass}>
                    Starting Life
                    <input
                      className={inputClass}
                      type="number"
                      min={1}
                      value={startingLife}
                      onChange={(event) => setStartingLife(Number(event.target.value) || 20)}
                    />
                  </label>
                  <label className={labelClass}>
                    Your Deck
                    <textarea
                      className={textareaClass}
                      value={createDeckText}
                      onChange={(event) => setCreateDeckText(event.target.value)}
                      placeholder={`Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Lightning Bolt\n4 Counterspell\n24 Island`}
                    />
                  </label>
                </div>

                <div className="grid gap-4 rounded-lg border border-[#243447] bg-[#09111a] p-4">
                  <label className={labelClass}>
                    Players
                    <select
                      className={inputClass}
                      value={desiredPlayers}
                      onChange={(event) => setDesiredPlayers(Number(event.target.value) || 2)}
                    >
                      <option value={2}>2 Players</option>
                      <option value={3}>3 Players</option>
                      <option value={4}>4 Players</option>
                    </select>
                  </label>
                  <div className="grid gap-1 text-[13px] leading-6 text-muted-foreground">
                    <span>
                      Host deck: {createDeckCount}/{LOBBY_DECK_SIZE} cards
                    </span>
                    <span>
                      Each player becomes ready after submitting exactly{" "}
                      {LOBBY_DECK_SIZE} main-deck cards.
                    </span>
                    <span>
                      The match auto-starts once every seat is filled and ready.
                    </span>
                  </div>
                  <Badge
                    variant="secondary"
                    className={`${pill} justify-center px-4 py-2`}
                    onClick={handleCreate}
                  >
                    Create Lobby
                  </Badge>
                </div>
              </div>
            ) : (
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px]">
                <div className="grid gap-4">
                  <label className={labelClass}>
                    Your Name
                    <input
                      className={inputClass}
                      value={joinName}
                      onChange={(event) => setJoinName(event.target.value)}
                      placeholder="Guest name"
                    />
                  </label>
                  <label className={labelClass}>
                    Lobby Code
                    <input
                      className={inputClass}
                      value={joinCode}
                      onChange={(event) => setJoinCode(event.target.value)}
                      placeholder="Host peer ID"
                    />
                  </label>
                  <label className={labelClass}>
                    Your Deck
                    <textarea
                      className={textareaClass}
                      value={joinDeckText}
                      onChange={(event) => setJoinDeckText(event.target.value)}
                      placeholder={`Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Thoughtseize\n4 Fatal Push\n24 Swamp`}
                    />
                  </label>
                </div>

                <div className="grid gap-4 rounded-lg border border-[#243447] bg-[#09111a] p-4">
                  <div className="grid gap-1 text-[13px] leading-6 text-muted-foreground">
                    <span>
                      Join deck: {joinDeckCount}/{LOBBY_DECK_SIZE} cards
                    </span>
                    <span>
                      You are only ready after the host receives exactly{" "}
                      {LOBBY_DECK_SIZE} main-deck cards from you.
                    </span>
                    <span>
                      You can still join early and finish the deck inside the lobby.
                    </span>
                  </div>
                  <Badge
                    variant="secondary"
                    className={`${pill} justify-center px-4 py-2`}
                    onClick={handleJoin}
                  >
                    Join Lobby
                  </Badge>
                </div>
              </div>
            )}
          </div>
        ) : (
          <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
            <div className="grid gap-4">
              <div className="grid gap-1 rounded-lg border border-[#243447] bg-[#09111a] p-4">
                <span className="text-[11px] uppercase tracking-[0.22em] text-[#7d97b4]">
                  Lobby Code
                </span>
                <div className="text-[28px] font-bold uppercase tracking-[0.14em] text-foreground">
                  {multiplayer.lobbyId || multiplayer.hostPeerId || "Connecting"}
                </div>
                <p className="text-[13px] text-muted-foreground">
                  {multiplayer.matchStarted
                    ? `Seat ${
                        multiplayer.localPlayerIndex != null
                          ? multiplayer.localPlayerIndex + 1
                          : "-"
                      } is active.`
                    : startPending
                      ? "All players are ready. Starting match."
                      : multiplayer.role === "host"
                        ? slotsRemaining > 0
                          ? `Share this code. ${slotsRemaining} slot${
                              slotsRemaining === 1 ? "" : "s"
                            } remaining.`
                          : `Waiting for ${
                              playerCount - readyPlayers
                            } player${
                              playerCount - readyPlayers === 1 ? "" : "s"
                            } to submit ${LOBBY_DECK_SIZE} cards.`
                        : localReady
                          ? "Ready. Waiting for the remaining players."
                          : `Paste exactly ${LOBBY_DECK_SIZE} main-deck cards to become ready.`}
                </p>
              </div>

              {!multiplayer.matchStarted ? (
                <div className="grid gap-2 rounded-lg border border-[#243447] bg-[#09111a] p-4">
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] uppercase tracking-[0.22em] text-[#7d97b4]">
                      Your Deck
                    </span>
                    <span className="text-[13px] text-muted-foreground">
                      {multiplayer.localDeckCount}/{LOBBY_DECK_SIZE}
                    </span>
                  </div>
                  <textarea
                    className={textareaClass}
                    value={multiplayer.localDeckText}
                    onChange={(event) => updateLobbyDeck(event.target.value)}
                    placeholder={`Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Swords to Plowshares\n4 Brainstorm\n24 Plains`}
                  />
                  <p className="text-[13px] leading-6 text-muted-foreground">
                    {localReady
                      ? "Ready. The host has your current 60-card main deck."
                      : `Not ready. The deck must contain exactly ${LOBBY_DECK_SIZE} main-deck cards.`}
                  </p>
                </div>
              ) : null}
            </div>

            <div className="grid gap-4">
              <div className="grid gap-2 rounded-lg border border-[#243447] bg-[#09111a] p-4">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] uppercase tracking-[0.22em] text-[#7d97b4]">
                    Players
                  </span>
                  <span className="text-[13px] text-muted-foreground">
                    {playerCount}/{multiplayer.desiredPlayers} seats, {readyPlayers} ready
                  </span>
                </div>
                {multiplayer.players.map((player) => (
                  <div
                    key={player.peerId}
                    className="flex items-center justify-between rounded-md border border-[#1f2d3d] bg-[#0b1118] px-3 py-2"
                  >
                    <span className="text-[14px] text-foreground">
                      {player.index + 1}. {player.name}
                    </span>
                    <span className="text-[12px] uppercase tracking-[0.18em] text-muted-foreground">
                      {formatPlayerStatus(player, multiplayer.localPeerId)}
                    </span>
                  </div>
                ))}
              </div>

              <div className="flex items-center justify-between gap-2">
                <span className="text-[13px] text-muted-foreground">
                  Starting life: {multiplayer.startingLife}
                </span>
                <Badge
                  variant="secondary"
                  className={pill}
                  onClick={() => leaveLobby("Lobby closed")}
                >
                  Leave Lobby
                </Badge>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
