import useUiText from "@/i18n/useUiText";
import PublicLobbySearch from './PublicLobbySearch';
import { PUBLIC_FORMATS, isRelayId, relayBaseUrl } from '@/lib/relay/formats';
import { validateFormatDeck, formatCatalogDate } from '@/lib/relay/format-legality';
import { useMemo, useState } from "react";
import LocalLobbySearch from "./LocalLobbySearch";
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
  MATCH_FORMAT_PLANECHASE,
  PARTNER_DECK_SIZE,
  normalizeMatchFormat,
  parseCommanderList,
  parseDeckList,
  parseSideboardList,
  readDefaultLobbyDeck,
} from "@/lib/decklists";
import {
  MULTIPLAYER_SECURITY_TRUSTED,
  MULTIPLAYER_SECURITY_VERIFIED,
  normalizeMultiplayerSecurityMode,
} from "@/lib/multiplayer-security";

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
const securityModeOptions = [
  {
    value: MULTIPLAYER_SECURITY_TRUSTED,
    label: "Trusted",
    description: "Fast friend-table mode with open decklists and no cryptographic anticheat.",
  },
  {
    value: MULTIPLAYER_SECURITY_VERIFIED,
    label: "Verified",
    description: "Cryptographic deck, hidden-info, and action verification with more setup time.",
  },
];

function formatName(format) {
  if (PUBLIC_FORMATS[format]) return PUBLIC_FORMATS[format].label;
  const normalized = normalizeMatchFormat(format);
  if (normalized === MATCH_FORMAT_COMMANDER) return "Commander";
  if (normalized === MATCH_FORMAT_PLANECHASE) return "Planechase";
  return "Normal";
}

function securityModeName(mode) {
  return normalizeMultiplayerSecurityMode(mode) === MULTIPLAYER_SECURITY_VERIFIED
    ? "Verified"
    : "Trusted";
}

function securityModeSummary(mode) {
  return normalizeMultiplayerSecurityMode(mode) === MULTIPLAYER_SECURITY_VERIFIED
    ? "Cryptographic anticheat and hidden-info proofs are enabled."
    : "Open decklists are used; cryptographic anticheat is off.";
}

function commanderDeckTarget(commanderCount) {
  return commanderCount === 2 ? PARTNER_DECK_SIZE : COMMANDER_DECK_SIZE;
}

function formatPlayerStatus(player, localPeerId, format) {
  if (player.connected === false) {
    const remainingMs = Number(player.disconnectRemainingMs ?? player.autoForfeitAtMs - Date.now());
    return `Offline ${formatCountdown(remainingMs)}`;
  }
  if (player.ready) return player.peerId === localPeerId ? "You / Ready" : "Ready";

  const normalizedFormat = normalizeMatchFormat(format);
  if (
    normalizedFormat === MATCH_FORMAT_COMMANDER
    || normalizedFormat === MATCH_FORMAT_PLANECHASE
  ) {
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

function formatCountdown(ms) {
  const totalSeconds = Math.max(0, Math.ceil(Number(ms || 0) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function offlinePlayerSummary(players) {
  const entries = Array.isArray(players) ? players : [];
  if (entries.length === 0) return "";
  return entries
    .map((player) => `${player.name} ${formatCountdown(player.remainingMs)}`)
    .join(", ");
}

function formatDeckRequirement(format) {
  if (PUBLIC_FORMATS[format] && format !== MATCH_FORMAT_COMMANDER) return "At least 60 main-deck cards; up to 15 sideboard cards. Format bans and copy limits apply.";
  const normalized = normalizeMatchFormat(format);
  if (normalized === MATCH_FORMAT_COMMANDER) {
    return `Submit a ${COMMANDER_DECK_SIZE}-card main deck plus 1 commander, or a ${PARTNER_DECK_SIZE}-card main deck plus 2 commanders.`;
  }
  if (normalized === MATCH_FORMAT_PLANECHASE) {
    return `Submit exactly ${LOBBY_DECK_SIZE} main-deck cards plus at least 10 uniquely named Plane or Phenomenon cards.`;
  }
  return `Submit exactly ${LOBBY_DECK_SIZE} main-deck cards.`;
}

export default function LobbyOverlay({
  onClose,
  defaultName = "Player",
  defaultStartingLife = 20,
  initialMode = "create",
  initialCreateFormat = MATCH_FORMAT_NORMAL,
  initialCreateName = "",
  initialCreateDeckText = "",
  initialCreateCommanderText = "",
  initialCreateSecurityMode = MULTIPLAYER_SECURITY_TRUSTED,
  initialJoinCode = "",
  initialJoinName = "",
  initialJoinDeckText = "",
  initialJoinCommanderText = "",
}) {
  const ui = useUiText();
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
  const lobbyDeckDefault = useMemo(() => readDefaultLobbyDeck(), []);
  const shouldDefaultCreateDeck =
    !String(initialCreateDeckText || "").trim()
    && !String(initialCreateCommanderText || "").trim();
  const shouldDefaultJoinDeck =
    !String(initialJoinDeckText || "").trim()
    && !String(initialJoinCommanderText || "").trim();
  const resolvedInitialCreateFormat =
    shouldDefaultCreateDeck && String(lobbyDeckDefault.commanderText || "").trim()
      ? MATCH_FORMAT_COMMANDER
      : normalizeMatchFormat(initialCreateFormat);
  const [transport, setTransport] = useState('peerjs');
  const [advertise, setAdvertise] = useState(true);
  const [mode, setMode] = useState(
    initialMode === "join" ? "join" : "create"
  );
  const [createFormat, setCreateFormat] = useState(
    resolvedInitialCreateFormat
  );
  const [createName, setCreateName] = useState(
    String(initialCreateName || defaultName)
  );
  const [joinName, setJoinName] = useState(String(initialJoinName || defaultName));
  const [joinCode, setJoinCode] = useState(String(initialJoinCode || ""));
  const [desiredPlayers, setDesiredPlayers] = useState(2);
  const [createSecurityMode, setCreateSecurityMode] = useState(
    normalizeMultiplayerSecurityMode(
      initialCreateSecurityMode,
      MULTIPLAYER_SECURITY_TRUSTED
    )
  );
  const [startingLife, setStartingLife] = useState(() => {
    const initialLife = Math.max(1, Number(defaultStartingLife) || 20);
    const normalizedFormat = resolvedInitialCreateFormat;
    if (normalizedFormat === MATCH_FORMAT_COMMANDER && initialLife === 20) {
      return 40;
    }
    if (normalizedFormat === MATCH_FORMAT_NORMAL && initialLife === 40) {
      return 20;
    }
    return initialLife;
  });
  const [createDeckText, setCreateDeckText] = useState(
    shouldDefaultCreateDeck
      ? String(lobbyDeckDefault.deckText || "")
      : String(initialCreateDeckText || "")
  );
  const [joinDeckText, setJoinDeckText] = useState(
    shouldDefaultJoinDeck
      ? String(lobbyDeckDefault.deckText || "")
      : String(initialJoinDeckText || "")
  );
  const [createCommanderText, setCreateCommanderText] = useState(
    shouldDefaultCreateDeck
      ? String(lobbyDeckDefault.commanderText || "")
      : String(initialCreateCommanderText || "")
  );
  const [joinCommanderText, setJoinCommanderText] = useState(
    shouldDefaultJoinDeck
      ? String(lobbyDeckDefault.commanderText || "")
      : String(initialJoinCommanderText || "")
  );
  const [inviteName, setInviteName] = useState("");
  const [inviteDeckText, setInviteDeckText] = useState("");
  const [inviteCommanderText, setInviteCommanderText] = useState("");

  const lobbyActive = multiplayer.mode !== "idle";
  const playerCount = multiplayer.players.length;
  const connectedPlayers = multiplayer.players.filter((player) => player.connected !== false).length;
  const readyPlayers = multiplayer.players.filter(
    (player) => player.connected !== false && player.ready
  ).length;
  const slotsRemaining = Math.max(0, multiplayer.desiredPlayers - connectedPlayers);
  const activeFormat = normalizeMatchFormat(multiplayer.format);

  const activeSecurityMode = normalizeMultiplayerSecurityMode(multiplayer.securityMode);
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
  const connectionWarnings = multiplayer.connectionWarnings || [];
  const offlinePlayers = connectionWarnings.filter((warning) => !warning.local);
  const shareLobbyCode = multiplayer.lobbyId || multiplayer.hostPeerId || "";
  const inviteLink = useMemo(
    () => buildLobbyInviteLink({
      lobbyId: shareLobbyCode,
      name: inviteName,
      deckText: inviteDeckText,
      commanderText:
        activeFormat === MATCH_FORMAT_COMMANDER
        || activeFormat === MATCH_FORMAT_PLANECHASE
          ? inviteCommanderText
          : "",
    }),
    [activeFormat, inviteCommanderText, inviteDeckText, inviteName, shareLobbyCode]
  );

  const publicDeckStatus = isRelayId(multiplayer.lobbyId) ? validateFormatDeck(activeFormat,
    parseDeckList(multiplayer.localDeckText || ''), parseCommanderList(multiplayer.localCommanderText || ''),
    parseSideboardList(multiplayer.localDeckText || '')) : null;

  const handleCreateFormatChange = (nextFormat) => {
    const normalized = normalizeMatchFormat(nextFormat);
    setCreateFormat(normalized);
    if (transport === 'websocket' && PUBLIC_FORMATS[normalized]) {
      setDesiredPlayers(PUBLIC_FORMATS[normalized].maxPlayers === 2 ? 2 : desiredPlayers);
      setStartingLife(PUBLIC_FORMATS[normalized].startingLife);
    }
    if (normalized === MATCH_FORMAT_PLANECHASE) {
      setCreateSecurityMode(MULTIPLAYER_SECURITY_TRUSTED);
    }
    setStartingLife((prev) => {
      if (normalized === MATCH_FORMAT_COMMANDER && prev === 20) return 40;
      if (normalized !== MATCH_FORMAT_COMMANDER && prev === 40) return 20;
      return prev;
    });
  };

  const handleCreate = () => {
    if (import.meta.env.VITE_LAN_LOBBY === "true"
      && createSecurityMode === MULTIPLAYER_SECURITY_VERIFIED && !globalThis.crypto?.subtle) {
      setStatus("Verified mode requires the LAN server's trusted HTTPS address. Use Trusted mode at this HTTP address.", true);
      return;
    }
    createLobby({
      transport,
      advertise,
      name: createName,
      desiredPlayers,
      startingLife,
      format: createFormat,
      securityMode:
        createFormat === MATCH_FORMAT_PLANECHASE
          ? MULTIPLAYER_SECURITY_TRUSTED
          : createSecurityMode,
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
          <div className="text-[11px] uppercase tracking-[0.24em] text-[#cdb27a]">{ui("Multiplayer")}</div>
          <SheetTitle className="text-[24px] uppercase tracking-[0.16em] text-foreground">
            {lobbyActive ? ui("Multiplayer Lobby") : mode === "join" ? ui("Join Lobby") : ui("Create Lobby")}
          </SheetTitle>
          <SheetDescription className="max-w-[46ch] text-[13px] leading-5">{ui("Host or join a multiplayer table, submit decks, and manage invite links from one place.")}</SheetDescription>
        </SheetHeader>

        <div className="lobby-sheet-body grid min-h-0 gap-4 p-4">
              {publicDeckStatus && <div className={panelClass} role="status">
                <span>{publicDeckStatus.ready ? ui('Your deck meets the format restrictions.') : publicDeckStatus.errors.slice(0, 5).join(' ')}</span>
                <small>{ui("Card legality snapshot:") + " "}{formatCatalogDate()?.slice(0, 10) || ui('loading')}</small>
              </div>}
          {!lobbyActive ? (
            <div className="grid gap-4">
              <div className="flex gap-2">
                <button
                  type="button"
                  className={`${modeTabClass} ${
                    mode === "create" ? "brightness-125" : "opacity-70"
                  }`}
                  aria-pressed={mode === "create"}
                  onClick={() => setMode("create")}
                >{ui("Create")}</button>
                <button
                  type="button"
                  className={`${modeTabClass} ${
                    mode === "join" ? "brightness-125" : "opacity-70"
                  }`}
                  aria-pressed={mode === "join"}
                  onClick={() => setMode("join")}
                >{ui("Join")}</button>
              </div>

              {mode === 'create' && <div className={panelClass}>
                <label className={labelClass}>{ui("Connection")}<select aria-label={ui("Connection")} className={inputClass} value={transport} onChange={event => {
                    const value = event.target.value;
                    setTransport(value);
                    if (value === 'websocket') {
                      const format = PUBLIC_FORMATS[createFormat] ? createFormat : 'modern';
                      setCreateFormat(format); setStartingLife(PUBLIC_FORMATS[format].startingLife);
                      setDesiredPlayers(PUBLIC_FORMATS[format].maxPlayers === 2 ? 2 : desiredPlayers);
                      setCreateSecurityMode(MULTIPLAYER_SECURITY_TRUSTED);
                    }
                  }}>
                    <option value="peerjs">{import.meta.env.VITE_LAN_LOBBY === 'true' ? ui('Local network') : ui('Peer-to-peer')}</option>
                    <option value="websocket" disabled={!relayBaseUrl()}>{ui("WebSocket lobby")}{!relayBaseUrl() ? ui(' (not configured)') : ''}</option>
                  </select>
                </label>
                {transport === 'websocket' && <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" checked={advertise} onChange={e => setAdvertise(e.target.checked)} />{ui("Advertise in public lobby search")}</label>}
                {transport === 'websocket' && <p className="text-sm text-muted-foreground">{ui("Format rules are enforced. Open decklists are shared with the table. Reopen this lobby link in the same browser to recover your seat. Play waits while the host is offline.")}</p>}
              </div>}
              {mode === "create" ? (
                <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
                  <div className="grid gap-4">
                    <div className="grid gap-4 md:grid-cols-2">
                      <label className={labelClass}>{ui("Your Name")}<input
                          className={inputClass}
                          value={createName}
                          onChange={(event) => setCreateName(event.target.value)}
                          placeholder={ui("Host name")}
                        />
                      </label>
                      <label className={labelClass}>{ui("Format")}<select
                          className={inputClass}
                          aria-label={ui("Format")}
                          value={createFormat}
                          onChange={(event) => handleCreateFormatChange(event.target.value)}
                        >
                          {transport === 'websocket' ? Object.values(PUBLIC_FORMATS).map(f => <option key={f.id} value={f.id}>{ui(f.label)}</option>) : <>
                            <option value={MATCH_FORMAT_NORMAL}>{ui("Normal")}</option>
                            <option value={MATCH_FORMAT_COMMANDER}>{ui("Commander")}</option>
                            <option value={MATCH_FORMAT_PLANECHASE}>{ui("Planechase")}</option>
                          </>}
                        </select>
                      </label>
                    </div>
                    <div className="grid gap-4 md:grid-cols-2">
                      <label className={labelClass}>{ui("Starting Life")}<input
                          className={inputClass}
                          type="number"
                          min={1}
                          value={startingLife}
                          disabled={transport === 'websocket'}
                          onChange={(event) => setStartingLife(Number(event.target.value) || 20)}
                        />
                      </label>
                      <label className={labelClass}>{ui("Players")}<select
                          className={inputClass}
                          aria-label={ui("Players")}
                          value={desiredPlayers}
                          disabled={transport === 'websocket' && PUBLIC_FORMATS[createFormat]?.maxPlayers === 2}
                          onChange={(event) => setDesiredPlayers(Number(event.target.value) || 2)}
                        >
                          <option value={2}>{ui("2 Players")}</option>
                          <option value={3}>{ui("3 Players")}</option>
                          <option value={4}>{ui("4 Players")}</option>
                        </select>
                      </label>
                    </div>
                    <fieldset className="grid gap-2">
                      <legend className="text-[12px] uppercase tracking-[0.18em] text-muted-foreground">{ui("Multiplayer Mode")}</legend>
                      <div className="grid gap-2 md:grid-cols-2">
                        {securityModeOptions.map((option) => {
                          if (transport === 'websocket' && option.value === MULTIPLAYER_SECURITY_VERIFIED) return null;
                          if (
                            createFormat === MATCH_FORMAT_PLANECHASE
                            && option.value === MULTIPLAYER_SECURITY_VERIFIED
                          ) {
                            return null;
                          }
                          const selected = createSecurityMode === option.value;
                          const needsHttps = import.meta.env.VITE_LAN_LOBBY === "true"
                            && option.value === MULTIPLAYER_SECURITY_VERIFIED && !globalThis.crypto?.subtle;
                          return (
                            <label
                              key={option.value}
                              className={`lobby-sheet-panel fantasy-sheet-section grid cursor-pointer gap-2 p-3 transition-all ${
                                selected ? "brightness-125" : "opacity-75"
                              }`}
                            >
                              <input
                                type="radio"
                                className="sr-only"
                                name="create-security-mode"
                                value={option.value}
                                checked={selected}
                                disabled={needsHttps}
                                onChange={() => setCreateSecurityMode(option.value)}
                              />
                              <span className="text-[13px] font-semibold uppercase tracking-[0.18em] text-foreground">
                                {ui(option.label)}
                              </span>
                              <span className="text-[13px] leading-5 text-muted-foreground">
                                {needsHttps ? ui("Open the LAN server's trusted HTTPS address to use Verified mode.") : option.description}
                              </span>
                            </label>
                          );
                        })}
                      </div>
                    </fieldset>
                    <label className={labelClass}>{ui("Main Deck")}<textarea
                        className={textareaClass}
                        value={createDeckText}
                        onChange={(event) => setCreateDeckText(event.target.value)}
                        placeholder={
                          ui(createFormat === MATCH_FORMAT_COMMANDER
                            ? `Paste a ${COMMANDER_DECK_SIZE}-card Commander main deck...\n\n1 Sol Ring\n1 Swords to Plowshares\n35 Plains`
                            : `Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Lightning Bolt\n4 Counterspell\n24 Island`)
                        }
                      />
                    </label>
                    {createFormat === MATCH_FORMAT_COMMANDER
                    || createFormat === MATCH_FORMAT_PLANECHASE ? (
                      <label className={labelClass}>
                        {createFormat === MATCH_FORMAT_PLANECHASE
                          ? ui("Planar Deck")
                          : ui("Commander(s)")}
                        <textarea
                          className={commanderTextareaClass}
                          value={createCommanderText}
                          onChange={(event) => setCreateCommanderText(event.target.value)}
                          placeholder={
                            ui(createFormat === MATCH_FORMAT_PLANECHASE
                              ? "1 The Aether Flues\n1 Spatial Merging\n1 The Great Forest\n..."
                              : "1 Atraxa, Praetors' Voice\nor\nTymna the Weaver\nKraum, Ludevic's Opus")
                          }
                        />
                      </label>
                    ) : null}
                  </div>

                  <div className={panelClass}>
                    <div className={infoTextClass}>
                      <span>{ui("Format:") + " "}{ui(formatName(createFormat))}</span>
                      <span>{ui("Main deck:")}{" "}
                        {createFormat === MATCH_FORMAT_COMMANDER
                          ? `${createDeckCount}/${createCommanderTarget}`
                          : `${createDeckCount}/${LOBBY_DECK_SIZE}`}
                      </span>
                      {createFormat === MATCH_FORMAT_COMMANDER ? (
                        <span>{ui("Commander(s):") + " "}{createCommanderCount}/1-2</span>
                      ) : createFormat === MATCH_FORMAT_PLANECHASE ? (
                        <span>{ui("Planar deck:") + " "}{createCommanderCount}/10+</span>
                      ) : null}
                      <span>{ui("Mode:") + " "}{ui(securityModeName(createSecurityMode))}</span>
                      <span>{ui(securityModeSummary(createSecurityMode))}</span>
                      <span>{ui(formatDeckRequirement(createFormat))}</span>
                      <span>{ui("The host can start the match once every seat is filled and ready.")}</span>
                    </div>
                    <Button
                      variant="secondary"
                      className={`${pill} ui-primary-action`}
                      onClick={handleCreate}
                    >{ui("Create Lobby")}</Button>
                  </div>
                </div>
              ) : (
                <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_300px]">
                  <div className="grid gap-4">
                    <div className="grid gap-4 md:grid-cols-2">
                      <label className={labelClass}>{ui("Your Name")}<input
                          className={inputClass}
                          value={joinName}
                          onChange={(event) => setJoinName(event.target.value)}
                          placeholder={ui("Guest name")}
                        />
                      </label>
                      <label className={labelClass}>{ui("Lobby Code")}<input
                          className={inputClass}
                          value={joinCode}
                          onChange={(event) => setJoinCode(event.target.value)}
                          placeholder={ui("Host lobby code")}
                        />
                      </label>
                    </div>
                    {relayBaseUrl() && <PublicLobbySearch onSelect={setJoinCode} />}
                    {import.meta.env.VITE_LAN_LOBBY === "true" && <LocalLobbySearch onSelect={setJoinCode} />}
                    <label className={labelClass}>{ui("Main Deck")}<textarea
                        className={textareaClass}
                        value={joinDeckText}
                        onChange={(event) => setJoinDeckText(event.target.value)}
                        placeholder={ui("Paste your main deck now or finish it inside the lobby.\n\nNormal and Planechase lobbies need {0} cards.\nCommander lobbies need {1} or {2} main-deck cards.", { 0: LOBBY_DECK_SIZE, 1: COMMANDER_DECK_SIZE, 2: PARTNER_DECK_SIZE })}
                      />
                    </label>
                    <label className={labelClass}>{ui("Commander(s) / Planar Deck")}<textarea
                        className={commanderTextareaClass}
                        value={joinCommanderText}
                        onChange={(event) => setJoinCommanderText(event.target.value)}
                        placeholder={ui("Optional until you see the host format.\nAdd 1 or 2 commanders for Commander, or at least 10 unique planar cards for Planechase.")}
                      />
                    </label>
                  </div>

                  <div className={panelClass}>
                    <div className={infoTextClass}>
                      <span>{ui("Main deck:") + " "}{joinDeckCount}{" " + ui("cards")}</span>
                      <span>{ui("Supplemental cards:") + " "}{joinCommanderCount}</span>
                      <span>{ui("Join first to see the host’s format and deck requirements.")}</span>
                      <span>{ui("You only become ready after the host receives a valid deck submission for that format.")}</span>
                    </div>
                    <Button
                      variant="secondary"
                      className={`${pill} ui-primary-action`}
                      disabled={!joinCode.trim()}
                      onClick={handleJoin}
                    >{ui("Join Lobby")}</Button>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
              <div className="grid gap-4">
                <div className="lobby-sheet-panel fantasy-sheet-section grid gap-1 p-4">
                  <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">{ui("Lobby Code")}</span>
                  <div className="lobby-sheet-code font-mono text-[24px] font-bold tracking-[0.04em] text-foreground">
                    {multiplayer.lobbyId || multiplayer.hostPeerId || ui("Connecting")}
                  </div>
                  <p className="text-[13px] text-muted-foreground">
                    {multiplayer.mode === "hosting"
                      ? ui("Registering lobby with PeerJS...")
                      : multiplayer.mode === "joining"
                        ? ui("Connecting to lobby host...")
                        : multiplayer.matchStarted
                          ? ui("Seat {0} is active.", { 0: multiplayer.localPlayerIndex != null
                                ? multiplayer.localPlayerIndex + 1
                                : "-" })
                          : startPending
                            ? ui("Starting match.")
                            : multiplayer.role === "host"
                              ? slotsRemaining > 0
                                ? ui("Share this code. {0} slot{1} remaining.", { 0: slotsRemaining, 1: slotsRemaining === 1 ? "" : "s" })
                                : canStartHostedMatch
                                  ? ui("All players are ready. Start the match when you're ready.")
                                  : ui("Waiting for {0} player{1} to submit a valid {2} deck.", { 0: playerCount - readyPlayers, 1: playerCount - readyPlayers === 1 ? "" : "s", 2: formatName(activeFormat) })
                              : localReady
                                ? readyPlayers === multiplayer.desiredPlayers
                                  ? ui("All players are ready. Waiting for the host to start.")
                                  : ui("Ready. Waiting for the remaining players.")
                                : formatDeckRequirement(activeFormat)}
                  </p>
                  <p className="text-[12px] uppercase tracking-[0.18em] text-[#c3a774]">{ui("Signaling:") + " "}{multiplayer.signalingServer || "0.peerjs.com:443"}
                  </p>
                  <p className="text-[12px] uppercase tracking-[0.18em] text-[#c3a774]">{ui("Mode:") + " "}{ui(securityModeName(activeSecurityMode))}
                  </p>
                  <p className="text-[13px] text-muted-foreground">
                    {ui(securityModeSummary(activeSecurityMode))}
                  </p>
                </div>

                {!multiplayer.matchStarted ? (
                  <div className="lobby-sheet-panel fantasy-sheet-section grid gap-3 p-4">
                    <div className="flex items-center justify-between gap-3">
                      <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">{ui("Invite Link")}</span>
                      <button
                        type="button"
                        disabled={!inviteLink}
                        className={`${startButtonClass} w-auto px-3 py-2`}
                        onClick={() => {
                          void handleCopyInviteLink();
                        }}
                      >{ui("Copy Link")}</button>
                    </div>
                    <label className={labelClass}>{ui("Invitee Name")}<input
                        className={inputClass}
                        value={inviteName}
                        onChange={(event) => setInviteName(event.target.value)}
                        placeholder={ui("Optional player name")}
                      />
                    </label>
                    <label className={labelClass}>{ui("Main Deck")}<textarea
                        className={textareaClass}
                        value={inviteDeckText}
                        onChange={(event) => setInviteDeckText(event.target.value)}
                        placeholder={
                          ui(activeFormat === MATCH_FORMAT_COMMANDER
                            ? `Optional ${COMMANDER_DECK_SIZE}-card or ${PARTNER_DECK_SIZE}-card main deck for this invitee`
                            : `Optional ${LOBBY_DECK_SIZE}-card main deck for this invitee`)
                        }
                      />
                    </label>
                    {activeFormat === MATCH_FORMAT_COMMANDER
                    || activeFormat === MATCH_FORMAT_PLANECHASE ? (
                      <label className={labelClass}>
                        {activeFormat === MATCH_FORMAT_PLANECHASE
                          ? ui("Planar Deck")
                          : ui("Commander(s)")}
                        <textarea
                          className={commanderTextareaClass}
                          value={inviteCommanderText}
                          onChange={(event) => setInviteCommanderText(event.target.value)}
                          placeholder={
                            ui(activeFormat === MATCH_FORMAT_PLANECHASE
                              ? "Optional planar deck for this invitee"
                              : "Optional until the invitee finalizes their commander choice")
                          }
                        />
                      </label>
                    ) : null}
                    <label className={labelClass}>{ui("Generated Link")}<textarea
                        className={`${commanderTextareaClass} min-h-[96px]`}
                        readOnly
                        value={inviteLink}
                        placeholder={ui("Invite link will appear once the lobby code is available")}
                      />
                    </label>
                    <div className={infoTextClass}>
                      <span>{ui("Includes the current lobby code plus any optional name, deck, and supplemental fields above.")}</span>
                      <span>{ui("Incomplete deck submissions still join the lobby and can be finished there before the player becomes ready.")}</span>
                    </div>
                  </div>
                ) : null}

                {!multiplayer.matchStarted ? (
                  <div className="lobby-sheet-panel fantasy-sheet-section grid gap-3 p-4">
                    <div className="flex items-center justify-between">
                      <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">{ui("Your Deck")}</span>
                      <span className="text-[13px] text-muted-foreground">{ui("Format:") + " "}{ui(formatName(activeFormat))}
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
                        ui(activeFormat === MATCH_FORMAT_COMMANDER
                          ? `Paste your Commander main deck...\n\n1 Sol Ring\n1 Brainstorm\n33 Island`
                          : `Paste a ${LOBBY_DECK_SIZE}-card main deck...\n\n4 Swords to Plowshares\n4 Brainstorm\n24 Plains`)
                      }
                    />
                    <div className={infoTextClass}>
                      <span>{ui("Main deck:")}{" "}
                        {activeFormat === MATCH_FORMAT_COMMANDER
                          ? `${multiplayer.localDeckCount}/${activeCommanderTarget}`
                          : `${multiplayer.localDeckCount}/${LOBBY_DECK_SIZE}`}
                      </span>
                      {activeFormat === MATCH_FORMAT_COMMANDER
                      || activeFormat === MATCH_FORMAT_PLANECHASE ? (
                        <>
                          <textarea
                            className={commanderTextareaClass}
                            disabled={startPending}
                            value={multiplayer.localCommanderText}
                            onChange={(event) =>
                              updateLobbyDeck({ commanderText: event.target.value })
                            }
                            placeholder={
                              ui(activeFormat === MATCH_FORMAT_PLANECHASE
                                ? "1 Plane or Phenomenon per line"
                                : "1 Commander\nor\nCommander One\nCommander Two")
                            }
                          />
                          <span>
                            {activeFormat === MATCH_FORMAT_PLANECHASE
                              ? ui("Planar deck: {0}/10+", { 0: multiplayer.localCommanderCount })
                              : ui("Commander(s): {0}/1-2", { 0: multiplayer.localCommanderCount })}
                          </span>
                        </>
                      ) : null}
                      <span>
                        {localReady
                          ? ui("Ready. The host has your current deck submission.")
                          : formatDeckRequirement(activeFormat)}
                      </span>
                    </div>
                  </div>
                ) : null}
              </div>

              <div className="grid gap-4">
                <div className="lobby-sheet-panel fantasy-sheet-section grid gap-2 p-4">
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] uppercase tracking-[0.22em] text-[#c3a774]">{ui("Players")}</span>
                    <span className="text-[13px] text-muted-foreground">
                      {playerCount}/{multiplayer.desiredPlayers}{" " + ui("seats,") + " "}{readyPlayers}{" " + ui("ready")}</span>
                  </div>
                  {multiplayer.players.map((player) => (
                    <div
                      key={player.peerId}
                      className={`lobby-sheet-player-row fantasy-sheet-stat flex items-center justify-between px-3 py-2 ${
                        player.connected === false ? "border-[#7d302f] bg-[#2b1114]/55" : ""
                      }`}
                    >
                      <span className="text-[14px] text-foreground">
                        {player.index + 1}. {player.name}
                      </span>
                      <span
                        className={`text-[12px] uppercase tracking-[0.18em] ${
                          player.connected === false ? "text-[#ffb8c0]" : "text-muted-foreground"
                        }`}
                      >
                        {ui(formatPlayerStatus(player, multiplayer.localPeerId, activeFormat))}
                      </span>
                    </div>
                  ))}
                </div>

                {multiplayer.matchStarted && offlinePlayers.length > 0 ? (
                  <div className="lobby-sheet-status border border-[#7d302f] bg-[#2b1114]/70 px-3 py-2 text-[13px] leading-5 text-[#ffb8c0]">
                    {offlinePlayers.length === 1
                      ? ui("{0} is disconnected. Wait {1} for the timeout policy.", { 0: offlinePlayers[0].name, 1: formatCountdown(offlinePlayers[0].remainingMs) })
                      : ui("{0} are disconnected. Wait for reconnects or timeout policy timers.", { 0: offlinePlayerSummary(offlinePlayers) })}
                  </div>
                ) : null}

                {!multiplayer.matchStarted && multiplayer.role === "host" ? (
                  <button
                    type="button"
                    disabled={!canStartHostedMatch || startPending}
                    className={startButtonClass}
                    onClick={() => {
                      void startHostedMatch();
                    }}
                  >
                    {startPending ? ui("Starting...") : ui("Start game")}
                  </button>
                ) : null}

                <div className="flex items-center justify-between gap-2">
                  <span className="text-[13px] text-muted-foreground">
                    {ui(formatName(activeFormat))}{" " + ui("• Starting life:") + " "}{multiplayer.startingLife}
                  </span>
                  <Button
                    variant="secondary"
                    className={pill}
                    onClick={() => leaveLobby("Lobby closed")}
                  >{ui("Leave Lobby")}</Button>
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
              {ui(status.msg)}
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
