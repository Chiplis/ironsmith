import usePrioritySettings from "@/hooks/usePrioritySettings";
import { useLayoutEffect, useRef, useState } from "react";
import { Axe, BrushCleaning, CircleCheck, FlagTriangleRight, Layers, RotateCcwSquare, Shield, Sparkles, Sunrise, Sunset, Sword, Swords, Zap } from 'lucide-react';
import { useGame } from '@/context/GameContext';
import { useI18n } from '@/i18n/I18nContext';
import useUiText from '@/i18n/useUiText';
import { normalizePhaseStep, PHASE_TRACK } from '@/lib/constants';
import { COMBAT_STEPS, priorityStepKey, priorityStopMode, DEFAULT_PRIORITY_STOPS } from '@/lib/priority-stops';
import { playerDisplayName } from '@/lib/player-display';
import { DEFAULT_PLAYER_ACCENT, getPlayerAccent } from '@/lib/player-colors';

const combatLabels = {
  BeginCombat: 'Beginning', DeclareAttackers: 'Attackers', DeclareBlockers: 'Blockers',
  FirstStrikeDamage: 'First strike', CombatDamage: 'Damage', EndCombat: 'End',
};
const combatIcons = {
  BeginCombat: FlagTriangleRight, DeclareAttackers: Swords, DeclareBlockers: Shield,
  FirstStrikeDamage: Zap, CombatDamage: Sword, EndCombat: CircleCheck,
};

// Each turn phase has its own icon; MTG has no official set.
const phaseIcons = {
  Untap: RotateCcwSquare, Upkeep: Sunrise, Draw: Layers, Main: Sparkles,
  Combat: Axe, Main2: Sparkles, End: Sunset, Cleanup: BrushCleaning,
};
const STOP_STATUS = { always: 'Stop every time', can_act: 'Stop when I can act' };

// One icon per phase. Between the icon and its label sits one dot per player,
// bordered in that player's color: hollow = no stop on their turn, white =
// stop if I can act, filled = stop every time. Clicking a dot cycles it.
export default function PhaseTrack({ compact = false, reserveCombatSpace = false }) {
  const { state, priorityStops = DEFAULT_PRIORITY_STOPS, cyclePriorityStop, priorityPolicy, playerAccentOverrides } = useGame();
  const priorityStatus = usePrioritySettings();
  const [previewCombat, setPreviewCombat] = useState(false);
  const bodyRef = useRef(null);
  const legendRef = useRef(null);
  // The legend starts at the same x as the mana symbols on the left edge of
  // the table. If that would run into the Untap icon (narrow layouts), it
  // stays in flow beside the track instead.
  useLayoutEffect(() => {
    const body = bodyRef.current;
    const legend = legendRef.current;
    if (!body || !legend) return undefined;
    // Measure the symbols themselves, not their padded chips or container.
    const manaLeft = () => Math.min(...[...document.querySelectorAll('.mana-pool-chip :is(img, svg)')]
      .map(node => node.getBoundingClientRect()).filter(rect => rect.width > 0).map(rect => rect.left));
    const measure = () => {
      legend.removeAttribute('data-anchored');
      legend.style.removeProperty('--phase-legend-left');
      if (!Number.isFinite(manaLeft())) return;
      legend.dataset.anchored = 'true';
      // Taking the legend out of flow can move both the track body and the
      // mana column, so settle the offset against the anchored layout.
      for (let pass = 0; pass < 3; pass += 1) {
        const offset = manaLeft() - body.getBoundingClientRect().left;
        if (Math.abs(offset - parseFloat(legend.style.getPropertyValue('--phase-legend-left') || 'NaN')) < 0.5) break;
        legend.style.setProperty('--phase-legend-left', `${offset}px`);
      }
      const icon = body.querySelector('.phase-track-stop-icon');
      if (icon && legend.getBoundingClientRect().right + 12 > icon.getBoundingClientRect().left) {
        legend.removeAttribute('data-anchored');
        legend.style.removeProperty('--phase-legend-left');
      }
    };
    measure();
    // The mana column and the track can slide without resizing (transitions,
    // late images and fonts), so re-measure after those as well. Resize
    // observations already arrive once per frame and are measured directly.
    let timer = 0;
    let disposed = false;
    const schedule = () => {
      if (disposed || timer) return;
      timer = setTimeout(() => { timer = 0; if (!disposed) measure(); }, 0);
    };
    const observer = new ResizeObserver(measure);
    for (const node of [body, body.closest('.topbar-phase-shell'), body.closest('.topbar-phase-shell')?.parentElement, document.documentElement]) {
      if (node) observer.observe(node);
    }
    const events = ['transitionend', 'animationend', 'load'];
    for (const type of events) document.addEventListener(type, schedule, true);
    document.fonts?.ready.then(schedule);
    return () => {
      disposed = true;
      observer.disconnect();
      for (const type of events) document.removeEventListener(type, schedule, true);
      clearTimeout(timer);
    };
  });
  const { t } = useI18n();
  const ui = useUiText();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const realPlayers = Boolean(state?.players?.length);
  const players = realPlayers ? state.players : [{ id: state?.active_player ?? 0, name: 'Turn' }];
  // Turn order starting from the local seat, matching TableCore's rotation.
  const seat = Math.max(0, players.findIndex(player => Number(player.id) === Number(state?.perspective)));
  const turnPlayers = [...players.slice(seat), ...players.slice(0, seat)];
  const self = turnPlayers[0];
  const playerName = player => playerDisplayName(players, player);
  const accentOf = player => (getPlayerAccent(players, player.id, state?.perspective, playerAccentOverrides) || DEFAULT_PLAYER_ACCENT).hex;
  const ownsTurn = player => Number(state?.active_player) === Number(player.id);
  const hasPriority = player => state?.decision?.kind === "priority"
    && Number(state.decision.player) === Number(player.id);
  // Each player's own turn count: the second player's first turn is Turn 1.
  // Older engine builds only report the game-wide number; assume an even rotation.
  const turnCount = state?.active_player_turn
    ?? (state?.turn_number != null ? Math.ceil(state.turn_number / Math.max(1, players.length)) : null);
  const phaseLabel = (entry, kind) => kind === 'step' ? combatLabels[entry] : t(`game.track.${entry}`, null, entry);
  const openPriorityMenu = (event, key, player) => {
    event.preventDefault();
    event.stopPropagation();
    window.dispatchEvent(new CustomEvent('ironsmith:priority-menu', { detail: { phase: key, player: player.id } }));
  };

  const renderDot = (entry, kind, player, disabled) => {
    const key = `${kind}:${entry}`;
    const mode = priorityStopMode(priorityStops, key, player.id);
    const label = phaseLabel(entry, kind);
    const status = STOP_STATUS[mode] || 'No stop';
    const name = realPlayers ? `${playerName(player)}'s turn` : 'Turn';
    const owner = realPlayers ? `${playerName(player)}: ` : '';
    return <button key={player.id} type="button" className="phase-track-player-stop" disabled={disabled}
      style={{ '--stop-accent': accentOf(player) }}
      data-stop-player={player.id} data-stop-phase={entry} data-stop={mode || 'off'}
      data-yield-target={priorityStatus?.yield?.mode === 'until' && priorityStatus.yield.key === key
        && Number(priorityStatus.yield.player) === Number(player.id) ? 'true' : undefined}
      data-active-turn={ownsTurn(player) ? 'true' : undefined}
      aria-label={`${owner}${label}: ${disabled ? 'No priority during untap' : status}`}
      aria-pressed={Boolean(mode)}
      title={`${label} — ${name}: ${disabled ? 'No priority during untap' : `${status}. Click to cycle: no stop, if I can act (white), every time (filled). Right-click for future yields and advanced stops.`}`}
      onContextMenu={event => openPriorityMenu(event, key, player)}
      onClick={() => cyclePriorityStop?.(key, realPlayers ? player.id : undefined)} />;
  };

  const renderStop = (entry, kind) => {
    const key = `${kind}:${entry}`;
    const Icon = (kind === 'step' ? combatIcons : phaseIcons)[entry] || Sparkles;
    const disabled = !state || entry === 'Untap';
    const label = phaseLabel(entry, kind);
    return <div className="phase-track-stop-button" data-stop-phase={entry} data-disabled={disabled ? 'true' : undefined}
      aria-expanded={entry === 'Combat' && kind === 'phase' ? active === 'Combat' || previewCombat : undefined}
      onContextMenu={disabled ? undefined : event => openPriorityMenu(event, key, self)}>
      <Icon className="phase-track-stop-icon" size={14} aria-hidden="true" />
      <span className="phase-track-player-stops" role="group" aria-label={`${label} stops`}>
        {turnPlayers.map(player => renderDot(entry, kind, player, disabled))}
      </span>
      <span className="phase-track-step-label">{label}</span>
    </div>;
  };

  const renderTrack = (entries, current, kind, concealed = false) => (
    <nav onMouseEnter={kind === 'step' ? () => setPreviewCombat(true) : undefined}
      className={`phase-track phase-track--sequence${kind === 'step' ? ' phase-track--combat' : ''}`}
      style={concealed ? { visibility: 'hidden', pointerEvents: 'none' } : undefined}
      aria-hidden={concealed || undefined} inert={concealed || undefined}
      data-compact={compact ? 'true' : 'false'} aria-label={kind === 'step' ? ui('Combat steps') : t('game.phaseTrack', null, 'Turn phases')}>
      <ol className="phase-track-steps">
        {entries.map((entry, index) => {
          const key = `${kind}:${entry}`;
          const currentEntry = entry === current;
          return <li key={key} className="phase-track-cell" data-phase-name={entry}
            onMouseEnter={entry === 'Combat' && kind === 'phase' ? () => setPreviewCombat(true) : undefined}
            data-phase-active={currentEntry ? 'true' : 'false'}
            data-phase-complete={entries.indexOf(current) > index ? 'true' : 'false'}
            aria-current={currentEntry ? 'step' : undefined}>
            {renderStop(entry, kind)}
            {entry === 'Combat' && kind === 'phase' && <span className="phase-track-empty-combat">
              {['attackers', 'blockers'].map(combatKind => {
                const attacking = combatKind === 'attackers';
                const enabled = (attacking ? priorityStatus.stopWithAttackers : priorityStatus.stopWithBlockers) !== false;
                const Icon = attacking ? Swords : Shield;
                const description = `When on, stop to declare ${combatKind} whenever you have legal ${combatKind}, even with no combat stops set. When off, normal phase stop settings apply.${attacking ? ' With no combat stops set, declare no attackers automatically (required attacks still stop).' : ''}`;
                return <button key={combatKind} type="button" className="phase-track-empty-combat-toggle"
                  data-combat-skip={combatKind} aria-label={`Stop with legal ${combatKind}`}
                  aria-pressed={enabled} disabled={!state} title={`${attacking ? 'Attackers' : 'Blockers'}: ${enabled ? 'On' : 'Off'}. ${description}`}
                  onClick={() => priorityPolicy?.setCombatParticipationStop(combatKind, !enabled, state)}>
                  <Icon size={12} aria-hidden="true" />
                </button>;
              })}
            </span>}
          </li>;
        })}
      </ol>
    </nav>
  );

  const showCombat = active === 'Combat' || previewCombat;
  return <div onMouseLeave={() => setPreviewCombat(false)}
    onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setPreviewCombat(false); }}
    onFocus={event => { if (event.target.closest?.('[data-phase-name="Combat"], .phase-track--combat')) setPreviewCombat(true); }}
    className="phase-track-group phase-track-group--players"
    data-player-count={turnPlayers.length}>
    <div className="phase-track-body" ref={bodyRef}>
      {realPlayers && <ul ref={legendRef} className="phase-track-legend" aria-label={ui('Players')}>
        {turnPlayers.map(player => <li key={player.id} className="phase-track-legend-player"
          style={{ '--stop-accent': accentOf(player) }} data-phase-player={player.id}
          data-active-turn={ownsTurn(player)} data-has-priority={hasPriority(player)}>
          <span className="phase-track-legend-dot" aria-hidden="true" />
          <span className="phase-track-player-name">{playerName(player)}</span>
          {ownsTurn(player) && <span className="phase-track-turn-cue">{turnCount != null ? ui('Turn {0}', { 0: turnCount }) : ui('Turn')}</span>}
          {hasPriority(player) && <span className="phase-track-priority-cue">{ui('Priority')}</span>}
        </li>)}
      </ul>}
      <div className="phase-track-tracks">
        {renderTrack(PHASE_TRACK, active, 'phase')}
        {(showCombat || reserveCombatSpace) && renderTrack(COMBAT_STEPS, priorityStepKey(state), 'step', !showCombat)}
      </div>
    </div>
  </div>;
}
