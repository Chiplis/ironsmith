import useUiText from "@/i18n/useUiText";
import { useMemo } from "react";
import { createPortal } from "react-dom";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import LobbyDeckEditor, { LobbyDeckCatalogPicker } from "@/components/layout/LobbyDeckEditor";
import RematchMainButton from "@/components/decisions/RematchMainButton";
import { useLobbyDeckOptions } from "@/lib/lobby-deck";
import { normalizeMatchFormat, parseCommanderList, parseDeckList } from "@/lib/decklists";

// Between games of a lobby match: every seat picks a deck the way it did when
// joining the lobby, marks it ready with the main decision button, and the
// host starts the next game once every deck is ready.
export default function RematchDeckView() {
  const ui = useUiText();
  const { t } = useI18n();
  const { multiplayer, updateRematchDeck } = useGame();
  const rematch = multiplayer?.rematch || {};
  const format = normalizeMatchFormat(multiplayer?.format);
  const deckText = String(rematch.localDeckText ?? "");
  const commanderText = String(rematch.localCommanderText ?? "");
  const deckCount = useMemo(() => parseDeckList(deckText).length, [deckText]);
  const commanderCount = useMemo(() => parseCommanderList(commanderText).length, [commanderText]);
  const deckOptions = useLobbyDeckOptions(multiplayer?.deckOptions);
  const players = rematch.players || [];
  const readyCount = players.filter((player) => player.ready).length;
  const starting = rematch.phase === "starting";
  const localReady = Boolean(rematch.localReady);
  const hostPeerId = multiplayer?.role === "host" ? multiplayer?.localPeerId : multiplayer?.hostPeerId;
  const topbarHost = typeof document !== "undefined"
    ? document.querySelector('[data-topbar-main-decision-host="true"]')
    : null;
  const onChange = (updates) => updateRematchDeck?.(updates);

  return (
    <div
      className="setup-screen rematch-deck-screen lobby-sheet-active flex h-full min-h-0 w-full flex-col gap-3 overflow-hidden bg-[linear-gradient(180deg,rgba(17,16,14,0.98),rgba(8,10,12,0.98))] px-3 py-3"
      data-rematch-deck-screen="true"
    >
      <div className="flex shrink-0 flex-wrap items-end justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-[18px] font-bold uppercase tracking-wide text-[#f2d9a3]">{t("game.nextGame")}</h1>
          <div className="mt-1 text-[12px] leading-snug text-muted-foreground">{t("game.chooseDeckNext")}</div>
        </div>
        <div className="text-[12px] font-semibold text-muted-foreground" data-rematch-ready-count="true">
          {t("game.playersReady", { ready: readyCount, total: players.length })}
        </div>
      </div>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
        <div className="lobby-sheet-active-grid">
          <div className="lobby-sheet-discovery">
            <LobbyDeckCatalogPicker format={format} disabled={starting} onChange={onChange} />
          </div>
          <div className="lobby-sheet-sidebar">
            <LobbyDeckEditor
              format={format}
              deckText={deckText}
              commanderText={commanderText}
              deckCount={deckCount}
              commanderCount={commanderCount}
              deckOptions={deckOptions}
              disabled={starting}
              readyText={localReady ? "Ready. The host has your current deck submission." : ""}
              onChange={onChange}
            />
            <div className="lobby-sheet-players lobby-sheet-panel fantasy-sheet-section grid gap-2 p-4">
              <div className="flex items-center justify-between">
                <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Players")}</span>
              </div>
              {players.map((player) => (
                <div
                  key={player.peerId}
                  className={`lobby-sheet-player-row fantasy-sheet-stat flex items-center justify-between px-3 py-2 ${
                    player.connected === false ? "bg-[#2b1114]" : ""
                  }`}
                  data-rematch-player-ready={player.ready ? "true" : "false"}
                >
                  <span className="text-[14px] text-foreground">
                    {Number(player.index) + 1}. {player.name}
                    {player.peerId === hostPeerId ? ` · ${ui("Host")}` : ""}
                  </span>
                  <span
                    className={`text-[12px] uppercase tracking-[0.18em] ${
                      player.connected === false ? "text-[#f0a9a0]" : "text-muted-foreground"
                    }`}
                  >
                    {player.connected === false
                      ? ui("Offline")
                      : player.ready
                        ? t("game.ready")
                        : ui("Choosing deck")}
                  </span>
                </div>
              ))}
            </div>
            {topbarHost ? null : <RematchMainButton className="h-11 w-full shrink-0" />}
          </div>
        </div>
      </div>
      {topbarHost ? createPortal(<RematchMainButton className="h-full w-full" />, topbarHost) : null}
    </div>
  );
}
