import { useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { copyTextToClipboard } from "@/lib/clipboard";
import {
  COMMANDER_DECK_SIZE,
  LOBBY_DECK_SIZE,
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  PARTNER_DECK_SIZE,
  normalizeMatchFormat,
  parseCommanderList,
  parseDeckList,
} from "@/lib/decklists";

const pill =
  "stone-pill inline-flex items-center justify-center rounded-none px-3 py-2 text-[13px] font-semibold uppercase tracking-[0.18em] transition-all select-none";
const inputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none";
const labelClass =
  "grid gap-1 text-[12px] uppercase tracking-[0.18em] text-muted-foreground";
const textareaClass =
  "fantasy-field min-h-[220px] w-full p-3 text-[14px] text-foreground outline-none font-mono resize-none";
const commanderTextareaClass =
  "fantasy-field lobby-sheet-commander-input min-h-[108px] w-full p-3 text-[14px] text-foreground outline-none font-mono resize-none";
const startButtonClass =
  "stone-pill inline-flex w-full items-center justify-center rounded-none px-4 py-3 text-[13px] font-semibold uppercase tracking-[0.2em] transition-all disabled:cursor-not-allowed disabled:opacity-50";
const panelClass = "lobby-sheet-panel fantasy-sheet-section grid gap-4 p-4";
const infoTextClass = "grid gap-1 text-[13px] leading-6 text-muted-foreground";
const modeTabClass =
  "stone-pill inline-flex items-center justify-center rounded-none px-4 py-2 text-[13px] font-semibold uppercase tracking-[0.18em] transition-all";

function formatName(format) {
  return normalizeMatchFormat(format) === MATCH_FORMAT_COMMANDER
    ? "Commander"
    : "Normal";
}

function commanderDeckTarget(commanderCount) {
  return commanderCount === 2 ? PARTNER_DECK_SIZE : COMMANDER_DECK_SIZE;
}

function formatPlayerStatus(player, localPeerId, format) {
  if (player.connected === false) return "Offline";
  if (player.ready) return player.peerId === localPeerId ? "You / Ready" : "Ready";

  if (normalizeMatchFormat(format) === MATCH_FORMAT_COMMANDER) {
    const mainCount = Number(player.deckCount || 0);
    const commanderCount = Number(player.commanderCount || 0);
    const prefix = player.peerId === localPeerId ? "You / " : "";
    return `${prefix}${mainCount} + ${commanderCount}`;
  }

  const deckCount = Number(player.deckCount || 0);
  return player.peerId === localPeerId
    ? `You / ${deckCount}/${LOBBY_DECK_SIZE}`
    : `${deckCount}/${LOBBY_DECK_SIZE}`;
}

function formatDeckRequirement(format) {
  return normalizeMatchFormat(format) === MATCH_FORMAT_COMMANDER
    ? `Submit a ${COMMANDER_DECK_SIZE}-card main deck plus 1 commander, or a ${PARTNER_DECK_SIZE}-card main deck plus 2 commanders.`
    : `Submit exactly ${LOBBY_DECK_SIZE} main-deck cards.`;
}

export default function LobbyOverlay({
  onClose,
  defaultName = "Player",
  defaultStartingLife = 20,
  initialMode = "create",
  initialJoinCode = "",
  initialJoinName = "",
  initialJoinDeckText = "",
  initialJoinCommanderText = "",
}) {
  const {
    multiplayer,
    canStartHostedMatch,
    createLobby,
    joinLobby,
    leaveLobby,
    startHostedMatch,
    updateLobbyDeck,
    status,
    setStatus,
  } = useGame();
  const [mode, setMode] = useState(
    initialMode === "join" ? "join" : "create"
  );
  const [createFormat, setCreateFormat] = useState(MATCH_FORMAT_NORMAL);
  const [createName, setCreateName] = useState(defaultName);
  const [joinName, setJoinName] = useState(String(initialJoinName || defaultName));
  const [joinCode, setJoinCode] = useState(String(initialJoinCode || ""));
  const [desiredPlayers, setDesiredPlayers] = useState(2);
  const [startingLife, setStartingLife] = useState(defaultStartingLife);
  const [createDeckText, setCreateDeckText] = useState("");
  const [joinDeckText, setJoinDeckText] = useState(String(initialJoinDeckText || ""));
  const [createCommanderText, setCreateCommanderText] = useState("");
  const [joinCommanderText, setJoinCommanderText] = useState(String(initialJoinCommanderText || ""));
  const [inviteName, setInviteName] = useState("");
  const [inviteDeckText, setInviteDeckText] = useState("");
  const [inviteCommanderText, setInviteCommanderText] = useState("");

  const lobbyActive = multiplayer.mode !== "idle";
  const playerCount = multiplayer.players.length;
  const readyPlayers = multiplayer.players.filter((player) => player.ready).length;
  const slotsRemaining = Math.max(0, multiplayer.desiredPlayers - playerCount);
  const activeFormat = normalizeMatchFormat(multiplayer.format);
  const createDeckCount = useMemo(
    () => parseDeckList(createDeckText).length,
    [createDeckText]
  );
  const joinDeckCount = useMemo(
    () => parseDeckList(joinDeckText).length,
    [joinDeckText]
  );
  const createCommanderCount = useMemo(
    () => parseCommanderList(createCommanderText).length,
    [createCommanderText]
  );
  const joinCommanderCount = useMemo(
    () => parseCommanderList(joinCommanderText).length,
    [joinCommanderText]
  );
  const localPlayer = multiplayer.players.find(
    (player) => player.peerId === multiplayer.localPeerId
  );
  const localReady = Boolean(localPlayer?.ready);
  const startPending = !multiplayer.matchStarted && multiplayer.mode === "starting";
  const activeCommanderTarget = commanderDeckTarget(multiplayer.localCommanderCount);
  const createCommanderTarget = commanderDeckTarget(createCommanderCount);
  const showLobbyStatus = Boolean(
    status?.msg
    && (
      status.isError
      || lobbyActive
      || /(lobby|peerjs|peer connection|signaling)/i.test(status.msg)
    )
  );
  const shareLobbyCode = multiplayer.lobbyId || multiplayer.hostPeerId || "";
  const inviteLink = useMemo(
    () => buildLobbyInviteLink({
      lobbyId: shareLobbyCode,
      name: inviteName,
      deckText: inviteDeckText,
      commanderText:
        activeFormat === MATCH_FORMAT_COMMANDER ? inviteCommanderText : "",
    }),
    [activeFormat, inviteCommanderText, inviteDeckText, inviteName, shareLobbyCode]
  );

  const handleCreateFormatChange = (nextFormat) => {
    const normalized = normalizeMatchFormat(nextFormat);
    setCreateFormat(normalized);
    setStartingLife((prev) => {
      if (normalized === MATCH_FORMAT_COMMANDER && prev === 20) return 40;
      if (normalized === MATCH_FORMAT_NORMAL && prev === 40) return 20;
      return prev;
    });
  };

  const handleCreate = () => {
    createLobby({
      name: createName,
      desiredPlayers,
      startingLife,
      format: createFormat,
      deckText: createDeckText,
      commanderText: createCommanderText,
    });
  };

  const handleJoin = () => {
    joinLobby({
      name: joinName,
      lobbyId: joinCode,
      deckText: joinDeckText,
      commanderText: joinCommanderText,
    });
  };

  const handleCopyInviteLink = async () => {
    if (!inviteLink) {
      setStatus("Lobby link is not available yet", true);
      return;
    }

    const copied = await copyTextToClipboard(inviteLink);
    if (copied) {
      setStatus("Copied invite link");
    } else {
      setStatus("Could not copy invite link", true);
    }
  };

  return (
    <Sheet open onOpenChange={(open) => {
      if (!open) onClose();
    }}>
      <SheetContent
        side="center"
        className="fantasy-sheet lobby-sheet flex max-h-[96vh] w-[min(96vw,1040px)] flex-col p-0"
      >
        <SheetHeader className="fantasy-sheet-header pr-12">
          <div className="text-[11px] uppercase tracking-[0.24em] text-[#cdb27a]">
            Multiplayer
          </div>
          <SheetTitle className="text-[24px] uppercase tracking-[0.16em] text-foreground">
            Create Lobby
          </SheetTitle>
          <SheetDescription className="max-w-[46ch] text-[13px] leading-5">
            Host or join a multiplayer table, submit decks, and manage invite links from one place.
          </SheetDescription>
        </SheetHeader>

        <div className="lobby-sheet-body grid min-h-0 gap-4 p-4">
          {!lobbyActive ? (
            <div className="grid gap-4">
              <div className="flex gap-2">
                <button
                  type="button"
                  className={`${modeTabClass} ${
                    mode === "create" ? "brightness-125" : "opacity-70"
                  }`}
                  onClick={() => setMode("create")}
                >
                  Create
                </button>
                <button
                  type="button"
                  className={`${modeTabClass} ${
                    mode === "join" ? "brightness-125" : "opacity-70"
                  }`}
                  onClick={() => setMode("join")}
                >
                  Join
                </button>
              </div>

              {mode === "create" ? (
                <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
                  <div className="grid gap-4">
                    <div className="grid gap-4 md:grid-cols-2">
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
                        Format
                        <select
                          className={inputClass}
                          value={createFormat}
                          onChange={(event) => handleCreateFormatChange(event.target.value)}
                        >
                          <option value={MATCH_FORMAT_NORMAL}>Normal</option>
                          <option value={MATCH_FORMAT_COMMANDER}>Commander</option>
                        </select>
                      </label>
                    </div>
                    <div className="grid gap-4 md:grid-cols-2">
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
                    </div>
                    <label className={labelClass}>
                      Main Deck
                      <textarea
                        className={textareaClass}
                        value={createDeckText}
                        onChange={(event) => setCreateDeckText(event.target.value)}
                        placeholder={
                          createFormat === MATCH_FORMAT_COMMANDER
                            ? `Paste a ${COMMANDER_DECK_SIZE}-card Commander main deck...\n\n1 Sol Ring\n1 Swords to Plowshares\n35 Plains`
                            : `Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Lightning Bolt\n4 Counterspell\n24 Island`
                        }
                      />
                    </label>
                    {createFormat === MATCH_FORMAT_COMMANDER ? (
                      <label className={labelClass}>
                        Commander(s)
                        <textarea
                          className={commanderTextareaClass}
                          value={createCommanderText}
                          onChange={(event) => setCreateCommanderText(event.target.value)}
                          placeholder={"1 Atraxa, Praetors' Voice\nor\nTymna the Weaver\nKraum, Ludevic's Opus"}
                        />
                      </label>
                    ) : null}
                  </div>

                  <div className={panelClass}>
                    <div className={infoTextClass}>
                      <span>Format: {formatName(createFormat)}</span>
                      <span>
                        Main deck:{" "}
                        {createFormat === MATCH_FORMAT_COMMANDER
                          ? `${createDeckCount}/${createCommanderTarget}`
                          : `${createDeckCount}/${LOBBY_DECK_SIZE}`}
                      </span>
                      {createFormat === MATCH_FORMAT_COMMANDER ? (
                        <span>Commander(s): {createCommanderCount}/1-2</span>
                      ) : null}
                      <span>{formatDeckRequirement(createFormat)}</span>
                      <span>
                        The host can start the match once every seat is filled and ready.
                      </span>
                    </div>
                    <Button
                      variant="secondary"
                      className={pill}
                      onClick={handleCreate}
                    >
                      Create Lobby
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
                  <div className="grid gap-4">
                    <div className="grid gap-4 md:grid-cols-2">
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
                    </div>
                    <label className={labelClass}>
                      Main Deck
                      <textarea
                        className={textareaClass}
                        value={joinDeckText}
                        onChange={(event) => setJoinDeckText(event.target.value)}
                        placeholder={`Paste your main deck now or finish it inside the lobby.\n\nNormal lobbies need ${LOBBY_DECK_SIZE} cards.\nCommander lobbies need ${COMMANDER_DECK_SIZE} or ${PARTNER_DECK_SIZE} main-deck cards.`}
                      />
                    </label>
                    <label className={labelClass}>
                      Commander(s)
                      <textarea
                        className={commanderTextareaClass}
                        value={joinCommanderText}
                        onChange={(event) => setJoinCommanderText(event.target.value)}
                        placeholder={"Optional until you see the host format.\nIf the lobby is Commander, add 1 or 2 commanders here."}
                      />
                    </label>
                  </div>

                  <div className={panelClass}>
                    <div className={infoTextClass}>
                      <span>Main deck: {joinDeckCount} cards</span>
                      <span>Commander(s): {joinCommanderCount}</span>
                      <span>
                        Join first, then the lobby will tell you whether the host chose Normal or Commander.
                      </span>
                      <span>
                        You only become ready after the host receives a valid deck submission for that format.
                      </span>
                    </div>
                    <Button
                      variant="secondary"
                      className={pill}
                      onClick={handleJoin}
                    >
                      Join Lobby
                    </Button>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
              <div className="grid gap-4">
                <div className="lobby-sheet-panel fantasy-sheet-section grid gap-1 p-4">
                  <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">
                    Lobby Code
                  </span>
                  <div className="lobby-sheet-code text-[28px] font-bold uppercase tracking-[0.14em] text-foreground">
                    {multiplayer.lobbyId || multiplayer.hostPeerId || "Connecting"}
                  </div>
                  <p className="text-[13px] text-muted-foreground">
                    {multiplayer.mode === "hosting"
                      ? "Registering lobby with PeerJS..."
                      : multiplayer.mode === "joining"
                        ? "Connecting to lobby host..."
                        : multiplayer.matchStarted
                          ? `Seat ${
                              multiplayer.localPlayerIndex != null
                                ? multiplayer.localPlayerIndex + 1
                                : "-"
                            } is active.`
                          : startPending
                            ? "Starting match."
                            : multiplayer.role === "host"
                              ? slotsRemaining > 0
                                ? `Share this code. ${slotsRemaining} slot${
                                    slotsRemaining === 1 ? "" : "s"
                                  } remaining.`
                                : canStartHostedMatch
                                  ? "All players are ready. Start the match when you're ready."
                                  : `Waiting for ${
                                      playerCount - readyPlayers
                                    } player${
                                      playerCount - readyPlayers === 1 ? "" : "s"
                                    } to submit a valid ${formatName(activeFormat)} deck.`
                              : localReady
                                ? readyPlayers === multiplayer.desiredPlayers
                                  ? "All players are ready. Waiting for the host to start."
                                  : "Ready. Waiting for the remaining players."
                                : formatDeckRequirement(activeFormat)}
                  </p>
                  <p className="text-[12px] uppercase tracking-[0.18em] text-[#c3a774]">
                    Signaling: {multiplayer.signalingServer || "0.peerjs.com:443"}
                  </p>
                </div>

                {!multiplayer.matchStarted ? (
                  <div className="lobby-sheet-panel fantasy-sheet-section grid gap-3 p-4">
                    <div className="flex items-center justify-between gap-3">
                      <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">
                        Invite Link
                      </span>
                      <button
                        type="button"
                        disabled={!inviteLink}
                        className={`${startButtonClass} w-auto px-3 py-2`}
                        onClick={() => {
                          void handleCopyInviteLink();
                        }}
                      >
                        Copy Link
                      </button>
                    </div>
                    <label className={labelClass}>
                      Invitee Name
                      <input
                        className={inputClass}
                        value={inviteName}
                        onChange={(event) => setInviteName(event.target.value)}
                        placeholder="Optional player name"
                      />
                    </label>
                    <label className={labelClass}>
                      Main Deck
                      <textarea
                        className={textareaClass}
                        value={inviteDeckText}
                        onChange={(event) => setInviteDeckText(event.target.value)}
                        placeholder={
                          activeFormat === MATCH_FORMAT_COMMANDER
                            ? `Optional ${COMMANDER_DECK_SIZE}-card or ${PARTNER_DECK_SIZE}-card main deck for this invitee`
                            : `Optional ${LOBBY_DECK_SIZE}-card main deck for this invitee`
                        }
                      />
                    </label>
                    {activeFormat === MATCH_FORMAT_COMMANDER ? (
                      <label className={labelClass}>
                        Commander(s)
                        <textarea
                          className={commanderTextareaClass}
                          value={inviteCommanderText}
                          onChange={(event) => setInviteCommanderText(event.target.value)}
                          placeholder={"Optional until the invitee finalizes their commander choice"}
                        />
                      </label>
                    ) : null}
                    <label className={labelClass}>
                      Generated Link
                      <textarea
                        className={`${commanderTextareaClass} min-h-[96px]`}
                        readOnly
                        value={inviteLink}
                        placeholder="Invite link will appear once the lobby code is available"
                      />
                    </label>
                    <div className={infoTextClass}>
                      <span>
                        Includes the current lobby code plus any optional name/deck/commander fields above.
                      </span>
                      <span>
                        Incomplete deck submissions still join the lobby and can be finished there before the player becomes ready.
                      </span>
                    </div>
                  </div>
                ) : null}

                {!multiplayer.matchStarted ? (
                  <div className="lobby-sheet-panel fantasy-sheet-section grid gap-3 p-4">
                    <div className="flex items-center justify-between">
                      <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">
                        Your Deck
                      </span>
                      <span className="text-[13px] text-muted-foreground">
                        Format: {formatName(activeFormat)}
                      </span>
                    </div>
                    <textarea
                      className={textareaClass}
                      disabled={startPending}
                      value={multiplayer.localDeckText}
                      onChange={(event) =>
                        updateLobbyDeck({ deckText: event.target.value })
                      }
                      placeholder={
                        activeFormat === MATCH_FORMAT_COMMANDER
                          ? `Paste your Commander main deck...\n\n1 Sol Ring\n1 Brainstorm\n33 Island`
                          : `Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Swords to Plowshares\n4 Brainstorm\n24 Plains`
                      }
                    />
                    <div className={infoTextClass}>
                      <span>
                        Main deck:{" "}
                        {activeFormat === MATCH_FORMAT_COMMANDER
                          ? `${multiplayer.localDeckCount}/${activeCommanderTarget}`
                          : `${multiplayer.localDeckCount}/${LOBBY_DECK_SIZE}`}
                      </span>
                      {activeFormat === MATCH_FORMAT_COMMANDER ? (
                        <>
                          <textarea
                            className={commanderTextareaClass}
                            disabled={startPending}
                            value={multiplayer.localCommanderText}
                            onChange={(event) =>
                              updateLobbyDeck({ commanderText: event.target.value })
                            }
                            placeholder={"1 Commander\nor\nCommander One\nCommander Two"}
                          />
                          <span>
                            Commander(s): {multiplayer.localCommanderCount}/1-2
                          </span>
                        </>
                      ) : null}
                      <span>
                        {localReady
                          ? "Ready. The host has your current deck submission."
                          : formatDeckRequirement(activeFormat)}
                      </span>
                    </div>
                  </div>
                ) : null}
              </div>

              <div className="grid gap-4">
                <div className="lobby-sheet-panel fantasy-sheet-section grid gap-2 p-4">
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">
                      Players
                    </span>
                    <span className="text-[13px] text-muted-foreground">
                      {playerCount}/{multiplayer.desiredPlayers} seats, {readyPlayers} ready
                    </span>
                  </div>
                  {multiplayer.players.map((player) => (
                    <div
                      key={player.peerId}
                      className="lobby-sheet-player-row fantasy-sheet-stat flex items-center justify-between px-3 py-2"
                    >
                      <span className="text-[14px] text-foreground">
                        {player.index + 1}. {player.name}
                      </span>
                      <span className="text-[12px] uppercase tracking-[0.18em] text-muted-foreground">
                        {formatPlayerStatus(player, multiplayer.localPeerId, activeFormat)}
                      </span>
                    </div>
                  ))}
                </div>

                {!multiplayer.matchStarted && multiplayer.role === "host" ? (
                  <button
                    type="button"
                    disabled={!canStartHostedMatch || startPending}
                    className={startButtonClass}
                    onClick={() => {
                      void startHostedMatch();
                    }}
                  >
                    {startPending ? "Starting..." : "Start game"}
                  </button>
                ) : null}

                <div className="flex items-center justify-between gap-2">
                  <span className="text-[13px] text-muted-foreground">
                    {formatName(activeFormat)} • Starting life: {multiplayer.startingLife}
                  </span>
                  <Button
                    variant="secondary"
                    className={pill}
                    onClick={() => leaveLobby("Lobby closed")}
                  >
                    Leave Lobby
                  </Button>
                </div>
              </div>
            </div>
          )}

          {showLobbyStatus ? (
            <div
              className={`lobby-sheet-status mt-4 border px-3 py-2 text-[13px] ${
                status.isError
                  ? "is-error text-[#ffb8c0]"
                  : "text-muted-foreground"
              }`}
            >
              {status.msg}
            </div>
          ) : null}
        </div>
      </SheetContent>
    </Sheet>
  );
}

function encodeBase64Utf8(text) {
  const value = String(text || "");
  if (!value || typeof window === "undefined") return "";

  try {
    const bytes = new TextEncoder().encode(value);
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    return window.btoa(binary)
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=+$/g, "");
  } catch {
    return "";
  }
}

function buildLobbyInviteLink({
  lobbyId = "",
  name = "",
  deckText = "",
  commanderText = "",
}) {
  if (typeof window === "undefined") return "";

  const trimmedLobbyId = String(lobbyId || "").trim();
  if (!trimmedLobbyId) return "";

  const url = new URL(window.location.href);
  url.search = "";
  url.hash = "";
  url.searchParams.set("lobby", trimmedLobbyId);

  const trimmedName = String(name || "").trim();
  const trimmedDeckText = String(deckText || "").trim();
  const trimmedCommanderText = String(commanderText || "").trim();

  if (trimmedName) {
    url.searchParams.set("name", trimmedName);
  }
  if (trimmedDeckText) {
    url.searchParams.set("deck", encodeBase64Utf8(trimmedDeckText));
  }
  if (trimmedCommanderText) {
    url.searchParams.set("commander", encodeBase64Utf8(trimmedCommanderText));
  }

  return url.toString();
}
