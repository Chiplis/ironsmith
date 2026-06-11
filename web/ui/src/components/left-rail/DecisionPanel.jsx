import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import { useGame } from "@/context/GameContext";
import { useHoveredObjectId } from "@/context/HoverContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import PeerWaitPopover, { PeerWaitButtonContent } from "@/components/decisions/PeerWaitPopover";
import useDeferredPeerWait from "@/hooks/useDeferredPeerWait";
import PhaseHelpPopover from "@/components/decisions/PhaseHelpPopover";
import PriorityPassButtonLabel from "@/components/decisions/PriorityPassButtonLabel";
import { normalizeDecisionText } from "@/components/decisions/decisionText";
import { KeywordHelpersProvider, SymbolText } from "@/lib/mana-symbols";
import { currentPriorityPhaseLabel, isMainPhase, nextPriorityAdvanceLabel } from "@/lib/constants";
import { useDecisionButtonAccent } from "@/lib/decision-button-style";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Flag, Undo2 } from "lucide-react";

const PRIORITY_ACTION_GROUPS = [
  { key: "play", label: "Play", kinds: ["play_land"] },
  { key: "cast", label: "Cast", kinds: ["cast_spell"] },
  { key: "untap", label: "Untap", kinds: ["untap_land"] },
  { key: "mana", label: "Mana", kinds: ["activate_mana_ability"] },
  { key: "activate", label: "Activate", kinds: ["activate_ability"] },
];
const BATTLEFIELD_HOVER_SUPPRESSED_KINDS = new Set(["activate_mana_ability", "activate_ability"]);

function zoneLabelFromAction(zone) {
  if (!zone) return "Unknown";
  switch (String(zone).toLowerCase()) {
    case "library": return "Library";
    case "hand": return "Hand";
    case "battlefield": return "Battlefield";
    case "graveyard": return "GY";
    case "exile": return "Exile";
    case "stack": return "Stack";
    case "command": return "CZ";
    default:
      return String(zone)
        .split(/[_\s]+/)
        .filter(Boolean)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" ");
  }
}

function formatPriorityActionLabel(action) {
  const label = action?.label || "";
  if (action?.kind === "play_land") {
    return `Play ${zoneLabelFromAction(action.from_zone)}`;
  }
  if (action?.kind === "cast_spell") {
    return `From ${zoneLabelFromAction(action.from_zone)}`;
  }
  if (action?.kind === "untap_land") {
    return "Untap";
  }
  if (action?.kind === "activate_ability" || action?.kind === "activate_mana_ability") {
    // "Activate Black Lotus: Add {R}{R}{R}." -> "Add {R}{R}{R}."
    const match = label.match(/^Activate\s+.+?:\s*(.+)$/i);
    if (match) return match[1];
  }
  return label;
}

function isBattlefieldObject(players, hoveredObjectId) {
  if (hoveredObjectId == null) return false;
  const hoveredId = String(hoveredObjectId);

  for (const player of players || []) {
    for (const card of player?.battlefield || []) {
      if (String(card?.id) === hoveredId) return true;
      if (Array.isArray(card?.member_ids)) {
        for (const memberId of card.member_ids) {
          if (String(memberId) === hoveredId) return true;
        }
      }
    }
  }
  return false;
}

function hoveredPriorityActionGroups(decision, hoveredObjectId, suppressBattlefieldAbilityOptions) {
  if (!decision || decision.kind !== "priority" || !hoveredObjectId) return [];

  const filtered = (decision.actions || []).filter(
    (action) =>
      action.kind !== "pass_priority"
      && action.object_id != null
      && String(action.object_id) === String(hoveredObjectId)
  );
  if (filtered.length === 0) return [];

  const visibleActions = suppressBattlefieldAbilityOptions
    ? filtered.filter((action) => !BATTLEFIELD_HOVER_SUPPRESSED_KINDS.has(action.kind))
    : filtered;
  if (visibleActions.length === 0) return [];

  const grouped = PRIORITY_ACTION_GROUPS
    .map((group) => ({
      key: group.key,
      label: group.label,
      actions: visibleActions.filter((action) => group.kinds.includes(action.kind)),
    }))
    .filter((group) => group.actions.length > 0);

  const groupedKinds = new Set(PRIORITY_ACTION_GROUPS.flatMap((group) => group.kinds));
  const otherActions = visibleActions.filter((action) => !groupedKinds.has(action.kind));
  if (otherActions.length > 0) {
    grouped.push({
      key: "other",
      label: "Other",
      actions: otherActions,
    });
  }

  return grouped;
}

export default function DecisionPanel({ inspectorOracleTextHeight = 0 }) {
  const {
    state,
    dispatch,
    cancelDecision,
    holdRule,
    setHoldRule,
    multiplayer,
    setStatus,
    startRematchSideboarding,
    readyForRematch,
    submitMultiplayerCommand,
    playerAccentOverrides,
  } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const [cancelling, setCancelling] = useState(false);
  const [surrendering, setSurrendering] = useState(false);
  const [visibleHoverGroups, setVisibleHoverGroups] = useState([]);
  const hideHoverGroupsTimerRef = useRef(null);
  const decision = state?.decision;
  const gameOver = state?.game_over || null;
  const rematch = multiplayer?.rematch || null;
  const rawPeerWait = multiplayer?.peerWait || null;
  const peerWait = useDeferredPeerWait(rawPeerWait);
  const peerWaiting = Boolean(peerWait);
  const peerWaitLocked = Boolean(rawPeerWait);
  const rematchSideboarding = rematch?.phase === "sideboarding";
  const rematchReady = Boolean(rematch?.localReady);
  const showGameOverPanel = Boolean(gameOver);
  const canPlayAgain = Boolean(
    gameOver
    && (multiplayer?.matchStarted || multiplayer?.mode === "in_match")
    && typeof startRematchSideboarding === "function"
  );
  const players = useMemo(() => state?.players || [], [state?.players]);
  const perspective = state?.perspective;
  const canAct = decision && samePlayerId(decision.player, perspective);
  const localPlayerIndex = multiplayer?.localPlayerIndex ?? perspective;
  const localPlayer = players.find((player) =>
    samePlayerId(player.id, localPlayerIndex)
    || samePlayerId(player.index, localPlayerIndex)
  ) || null;
  const localPlayerLeft = Boolean(
    localPlayer?.has_lost
    || localPlayer?.hasLost
    || localPlayer?.has_left_game
    || localPlayer?.hasLeftGame
  );

  const decisionPlayer = decision
    ? players.find((p) => samePlayerId(p.id, decision.player))
    : null;

  const gameOverText = gameOver?.kind === "winner"
    ? `${gameOver.name || `Player ${Number(gameOver.player || 0) + 1}`} wins`
    : gameOver?.kind === "draw"
      ? "The game is a draw"
      : gameOver?.kind === "remaining"
        ? "Game complete"
        : "";

  const metaText = gameOverText || (decision
    ? `${playerDisplayName(players, decisionPlayer)} · ${decision.reason || decision.kind}`
    : "No pending action");

  const isPriorityDecision = decision?.kind === "priority";
  const passAction = isPriorityDecision
    ? (decision.actions || []).find((action) => action.kind === "pass_priority")
    : null;
  const stackSize = state?.stack_size || 0;
  const holdingPriority = holdRule === "always";
  const hasCustomPassLabel = !!passAction?.label && passAction.label !== "Pass priority";
  const resolvingStackPriority = stackSize > 0 && !hasCustomPassLabel;
  const passAdvanceLabel = resolvingStackPriority
    ? ""
    : (hasCustomPassLabel
        ? ""
        : holdingPriority
          ? passAction?.label || "Pass priority"
        : `→ ${nextPriorityAdvanceLabel(state?.phase, state?.step, stackSize)}`);
  const passCurrentLabel = resolvingStackPriority
    ? "Resolve"
    : (hasCustomPassLabel ? passAction.label : currentPriorityPhaseLabel(state?.phase, state?.step));
  const passHelpAdvanceLabel = resolvingStackPriority
    ? "Resolve"
    : (hasCustomPassLabel ? passAction.label : passAdvanceLabel);
  const { style: decisionButtonStyle, isLocal: localDecisionButton } =
    useDecisionButtonAccent(state, decision, playerAccentOverrides);
  const showSurrender = Boolean(
    multiplayer?.matchStarted
    && !showGameOverPanel
    && !localPlayerLeft
    && submitMultiplayerCommand
  );
  const canSurrender = Boolean(
    showSurrender
    && !surrendering
    && !multiplayer?.submittingAction
    && isPriorityDecision
    && canAct
    && samePlayerId(state?.active_player, localPlayerIndex)
    && Number(stackSize || 0) === 0
    && isMainPhase(state?.phase)
  );

  const undoAvailable = !!state?.cancelable && (!decision || canAct);
  const undoDisabled = cancelling || !undoAvailable;
  const suppressBattlefieldAbilityOptions = useMemo(
    () => isBattlefieldObject(players, hoveredObjectId),
    [players, hoveredObjectId]
  );
  const hoverGroups = useMemo(
    () => hoveredPriorityActionGroups(
      decision,
      hoveredObjectId,
      suppressBattlefieldAbilityOptions
    ),
    [decision, hoveredObjectId, suppressBattlefieldAbilityOptions]
  );
  const showHoverOptions = hoverGroups.length > 0;

  useEffect(() => {
    if (hideHoverGroupsTimerRef.current) {
      clearTimeout(hideHoverGroupsTimerRef.current);
      hideHoverGroupsTimerRef.current = null;
    }

    if (showHoverOptions) {
      hideHoverGroupsTimerRef.current = setTimeout(() => {
        setVisibleHoverGroups(hoverGroups);
        hideHoverGroupsTimerRef.current = null;
      }, 0);
      return;
    }

    hideHoverGroupsTimerRef.current = setTimeout(() => {
      setVisibleHoverGroups([]);
      hideHoverGroupsTimerRef.current = null;
    }, 220);
  }, [hoverGroups, showHoverOptions]);

  useEffect(() => {
    return () => {
      if (hideHoverGroupsTimerRef.current) {
        clearTimeout(hideHoverGroupsTimerRef.current);
        hideHoverGroupsTimerRef.current = null;
      }
    };
  }, []);

  const handleCancel = useCallback(() => {
    setCancelling(true);
    setTimeout(() => {
      cancelDecision();
      setCancelling(false);
    }, 350);
  }, [cancelDecision]);

  const handleRematchClick = useCallback(() => {
    if (!canPlayAgain) return;
    if (rematchSideboarding) {
      if (!rematchReady) readyForRematch?.();
      return;
    }
    startRematchSideboarding?.();
  }, [
    readyForRematch,
    canPlayAgain,
    rematchReady,
    rematchSideboarding,
    startRematchSideboarding,
  ]);

  const handleSurrender = useCallback(async () => {
    if (!canSurrender) return;
    const playerName = playerDisplayName(players, localPlayer) || `Player ${Number(localPlayerIndex) + 1}`;
    if (!window.confirm(`Surrender as ${playerName}? This will be signed and broadcast to the match.`)) {
      return;
    }
    setSurrendering(true);
    try {
      await submitMultiplayerCommand?.(
        {
          type: "forfeit_player",
          player: Number(localPlayerIndex),
          reason: "surrender",
        },
        `${playerName} surrendered`
      );
    } catch (err) {
      setStatus?.(`Surrender failed: ${err?.message || err}`, true);
    } finally {
      setSurrendering(false);
    }
  }, [
    canSurrender,
    localPlayer,
    localPlayerIndex,
    players,
    setStatus,
    submitMultiplayerCommand,
  ]);

  return (
    <KeywordHelpersProvider enabled={false}>
      <section className="relative z-30 flex h-full min-h-0 flex-1 flex-col overflow-visible border-t border-[rgba(128,107,78,0.46)] bg-[linear-gradient(180deg,rgba(41,35,31,0.98),rgba(15,14,14,0.98))] backdrop-blur-[1.5px]">
      {/* Cancel flash overlay */}
      {cancelling && (
        <div
          className="absolute inset-0 z-10 pointer-events-none rounded"
          style={{ animation: "cancel-flash 350ms ease-out forwards" }}
        />
      )}

      <div className="relative z-20 flex h-full min-h-0 flex-1 flex-col overflow-visible">
        <div
          className="w-full min-h-0 flex-1 overflow-visible px-1.5 pt-1.5"
          style={cancelling ? { animation: "cancel-slide-out 350ms ease-in forwards" } : undefined}
        >
          {showGameOverPanel ? (
            <div className="flex h-full min-h-[110px] flex-col justify-center gap-2 px-2 py-2">
              <div className="text-[11px] font-bold uppercase tracking-wider text-[#d8bf7a]">
                Game Over
              </div>
              <div className="text-[16px] font-bold leading-tight text-[#f2d9a3]">
                {gameOverText}
              </div>
              {rematchSideboarding ? (
                <div className="text-[12px] leading-snug text-muted-foreground">
                  Sideboard for the next game, then mark yourself ready.
                </div>
              ) : (
                <div className="text-[12px] leading-snug text-muted-foreground">
                  {canPlayAgain ? "Start a rematch with the same seats." : "The game has ended."}
                </div>
              )}
            </div>
          ) : decision ? (
            <DecisionRouter
              decision={decision}
              canAct={canAct}
              inspectorOracleTextHeight={inspectorOracleTextHeight}
            />
          ) : (
            <div className="text-muted-foreground text-[13px] italic">
              Waiting...
            </div>
          )}
        </div>

        <div className="relative shrink-0 px-1.5 py-1 border-t border-[rgba(128,107,78,0.36)]">
          {isPriorityDecision && (
            <div
              className={`overflow-hidden pointer-events-none transition-all duration-200 ease-out ${
                showHoverOptions
                  ? "max-h-[280px] opacity-100 translate-y-0 pb-1"
                  : "max-h-0 opacity-0 translate-y-1 pb-0"
              }`}
            >
              <div
                className="px-0 py-1 bg-[#070f17]"
              >
                {visibleHoverGroups.length > 0 && (
                  <div className="grid gap-1.5 max-h-[280px] overflow-y-auto">
                    {visibleHoverGroups.map((group, groupIndex) => (
                      <div
                        key={group.key}
                        className={groupIndex > 0 ? "pt-1 border-t border-[#2a3647]" : ""}
                      >
                        <h4 className="text-[11px] uppercase tracking-wider font-bold text-[#c6ddff]">
                          {group.label}
                        </h4>
                        <div className="grid gap-0.5 mt-0.5">
                          {group.actions.map((action) => (
                            <div key={action.index} className="text-[13px] leading-snug text-[#d6e6fb]">
                              <SymbolText text={normalizeDecisionText(formatPriorityActionLabel(action))} />
                            </div>
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          )}

          {showGameOverPanel && canPlayAgain && (
            <div className="pb-1">
              <PeerWaitPopover peerWait={peerWait}>
                <Button
                  variant="ghost"
                  size="sm"
                  className="decision-neon-button decision-main-button pass-priority-btn h-auto min-h-10 w-full shrink-0 justify-start px-3 py-1.5 text-left text-[15px] font-bold uppercase whitespace-normal"
                  aria-disabled={peerWaitLocked || (rematchSideboarding && rematchReady)}
                  disabled={peerWaiting ? false : (rematchSideboarding && rematchReady)}
                  onClick={() => {
                    if (peerWaitLocked) return;
                    handleRematchClick();
                  }}
                >
                  {peerWaiting ? (
                    <PeerWaitButtonContent />
                  ) : (
                    rematchSideboarding
                      ? rematchReady ? "Waiting for players" : "Ready"
                      : "Play again"
                  )}
                </Button>
              </PeerWaitPopover>
            </div>
          )}

          {!showGameOverPanel && isPriorityDecision && passAction && (
            <div className="pb-1">
              <div className="relative">
                <PeerWaitPopover peerWait={peerWait}>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="decision-neon-button decision-main-button pass-priority-btn group h-auto min-h-10 w-full shrink-0 justify-start px-3 py-1.5 pr-9 text-left text-[15px] font-bold uppercase whitespace-normal"
                    style={decisionButtonStyle}
                    data-local-action={localDecisionButton ? "true" : "false"}
                    aria-disabled={peerWaitLocked || !canAct}
                    onClick={() => {
                      if (peerWaitLocked || !canAct) return;
                      dispatch(
                        { type: "priority_action", action_index: passAction.index, action_ref: passAction.action_ref },
                        passAction.label
                      );
                    }}
                  >
                    {peerWaiting ? (
                      <PeerWaitButtonContent />
                    ) : (
                      <PriorityPassButtonLabel
                        currentLabel={passCurrentLabel}
                        advanceLabel={passAdvanceLabel}
                      />
                    )}
                  </Button>
                </PeerWaitPopover>
                {!peerWaiting && (
                  <PhaseHelpPopover
                    state={state}
                    decision={decision}
                    advanceLabel={passHelpAdvanceLabel}
                    className="absolute right-1.5 top-1/2 z-20 -translate-y-1/2"
                  />
                )}
              </div>
            </div>
          )}

          <div className="flex items-center gap-1 shrink-0 flex-wrap">
            <h3 className="m-0 text-[12px] font-bold whitespace-nowrap uppercase tracking-wider text-[#8ec4ff]">Action</h3>
            <span className="text-muted-foreground text-[11px] truncate flex-1 min-w-0">{metaText}</span>
            <div className="flex items-center gap-1">
              {showSurrender ? (
                <Button
                  variant="ghost"
                  size="sm"
                  className={`h-5 w-5 p-0 shrink-0 transition-all ${
                    canSurrender
                      ? "text-[#f7b267]/70 hover:bg-[#f7b267]/10 hover:text-[#ffd7a1] hover:shadow-[0_0_8px_rgba(247,178,103,0.15)]"
                      : "text-muted-foreground/35 opacity-65"
                  }`}
                  disabled={!canSurrender}
                  onClick={handleSurrender}
                  title={canSurrender ? "Surrender" : "Surrender is available at sorcery speed"}
                  aria-label={canSurrender ? "Surrender" : "Surrender unavailable"}
                >
                  <Flag className="h-3.5 w-3.5" />
                </Button>
              ) : null}
              <Button
                variant="ghost"
                size="sm"
                className={`h-5 w-5 p-0 shrink-0 transition-all ${
                  undoAvailable
                    ? "text-[#f76969]/60 hover:text-[#f76969] hover:bg-[#f76969]/10 hover:shadow-[0_0_8px_rgba(247,105,105,0.15)]"
                    : "text-muted-foreground/35 opacity-65"
                }`}
                disabled={undoDisabled}
                onClick={handleCancel}
                title={undoAvailable ? "Undo" : "Undo unavailable"}
                aria-label={undoAvailable ? "Undo" : "Undo unavailable"}
              >
                <Undo2 className="h-3.5 w-3.5" />
              </Button>
              <label className="flex items-center gap-1 shrink-0 text-[11px] uppercase tracking-wider cursor-pointer text-muted-foreground hover:text-foreground transition-colors">
                <Checkbox
                  checked={holdRule === "always"}
                  onCheckedChange={(v) => setHoldRule(v ? "always" : "never")}
                  className="h-3 w-3"
                />
                Hold
              </label>
            </div>
          </div>
        </div>
      </div>
      </section>
    </KeywordHelpersProvider>
  );
}
