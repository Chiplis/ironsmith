import usePrioritySettings from "@/hooks/usePrioritySettings";
import { useState } from "react";
import { Axe, BrushCleaning, CircleCheck, FlagTriangleRight, Hand, Layers, RotateCcwSquare, Shield, Sparkles, Sunrise, Sunset, Sword, Swords, Zap } from 'lucide-react';
import { useGame } from '@/context/GameContext';
import { useI18n } from '@/i18n/I18nContext';
import useUiText from '@/i18n/useUiText';
import { normalizePhaseStep, PHASE_TRACK } from '@/lib/constants';
import { COMBAT_STEPS, priorityStepKey, priorityStopMode, DEFAULT_PRIORITY_STOPS } from '@/lib/priority-stops';
import { playerDisplayName } from '@/lib/player-display';

const combatLabels = {
  BeginCombat: 'Beginning', DeclareAttackers: 'Attackers', DeclareBlockers: 'Blockers',
  FirstStrikeDamage: 'First strike', CombatDamage: 'Damage', EndCombat: 'End',
};
const combatIcons = {
  BeginCombat: FlagTriangleRight, DeclareAttackers: Swords, DeclareBlockers: Shield,
  FirstStrikeDamage: Zap, CombatDamage: Sword, EndCombat: CircleCheck,
};

// Each turn phase has its own icon; MTG has no official set. The stop state
// (none / if I can act / every time) colors and fills the icon.
const phaseIcons = {
  Untap: RotateCcwSquare, Upkeep: Sunrise, Draw: Layers, Main: Sparkles,
  Combat: Axe, Main2: Sparkles, End: Sunset, Cleanup: BrushCleaning,
};

export default function PhaseTrack({ compact = false, reserveCombatSpace = false }) {
  const { state, priorityStops = DEFAULT_PRIORITY_STOPS, cyclePriorityStop, priorityPolicy } = useGame();
  const priorityStatus = usePrioritySettings();
  const [locked, setLocked] = useState(false);
  const [previewCombatPlayer, setPreviewCombatPlayer] = useState(null);
  const { t } = useI18n();
  const ui = useUiText();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const players = state?.players?.length ? state.players : [{ id: state?.active_player ?? 0, name: 'Turn' }];
  // Match TableCore's seat rotation, so each opponent stays over their field.
  const seat = Math.max(0, players.findIndex(player => Number(player.id) === Number(state?.perspective)));
  const turnPlayers = [...players.slice(seat), ...players.slice(0, seat)];
  const [self, ...opponents] = turnPlayers;
  const duel = players.length === 2;
  const multiplayer = players.length > 2;
  const playerName = player => playerDisplayName(players, player);
  const ownsTurn = player => Number(state?.active_player) === Number(player.id);
  const hasPriority = player => state?.decision?.kind === "priority"
    && Number(state.decision.player) === Number(player.id);
  // Each player's own turn count: the second player's first turn is Turn 1.
  // Older engine builds only report the game-wide number; assume an even rotation.
  const turnCount = state?.active_player_turn
    ?? (state?.turn_number != null ? Math.ceil(state.turn_number / Math.max(1, players.length)) : null);
  const phaseLabel = (entry, kind) => kind === 'step' ? combatLabels[entry] : t(`game.track.${entry}`, null, entry);

  const renderStop = (entry, kind, player) => {
    const key = `${kind}:${entry}`;
    const Icon = (kind === 'step' ? combatIcons : phaseIcons)[entry] || Sparkles;
    const mode = priorityStopMode(priorityStops, key, player.id);
    const disabled = !state || entry === 'Untap';
    const label = phaseLabel(entry, kind);
    const status = mode === 'always' ? 'Stop every time' : mode ? 'Stop when I can act' : 'No stop';
    return <button type="button" className="phase-track-stop-button" disabled={disabled}
      onFocus={entry === 'Combat' ? () => setPreviewCombatPlayer(player.id) : undefined}
      aria-expanded={entry === 'Combat' ? active === 'Combat' || previewCombatPlayer === player.id : undefined}
      data-stop-player={player.id} data-stop-phase={entry} data-stop={mode || 'off'}
      data-yield-target={priorityStatus?.yield?.mode === 'until' && priorityStatus.yield.key === key
        && Number(priorityStatus.yield.player) === Number(player.id) ? 'true' : undefined}
      data-opponent-stop={duel && Number(player.id) !== Number(self.id) ? 'true' : undefined}
      aria-label={`${state?.players?.length ? `${playerName(player)}: ` : ''}${label}: ${disabled ? 'No priority during untap' : status}`}
      aria-pressed={Boolean(mode)} title={`${playerName(player)} — ${label}: ${disabled ? 'No priority during untap' : `${status}. Click to cycle: no stop, if I can act, every time. Right-click for future yields and advanced stops.`}`}
      onContextMenu={event => {
        event.preventDefault();
        window.dispatchEvent(new CustomEvent('ironsmith:priority-menu', { detail: { phase: key, player: player.id } }));
      }}
      onClick={() => { if (!locked) {
        cyclePriorityStop?.(key, state?.players?.length ? player.id : undefined);
      } }}>
      <Icon className="phase-track-stop-icon" size={14} aria-hidden="true" />
      <span className="phase-track-step-label">{label}</span>
    </button>;
  };

  const renderHeading = player => <header className="phase-track-player-heading">
    {duel && <Hand size={12} aria-hidden="true" className={player.id === self.id ? undefined : 'phase-track-opponent-hand'} />}
    <span className="phase-track-player-name">{playerName(player)}</span>
    {ownsTurn(player) && <span className="phase-track-turn-cue">{turnCount != null ? ui('Turn {0}', { 0: turnCount }) : ui('Turn')}</span>}
    {hasPriority(player) && <span className="phase-track-priority-cue">{ui('Priority')}</span>}
  </header>;

  const renderTrack = (entries, current, kind, player, concealed = false) => (
    <nav onMouseEnter={kind === 'step' ? () => setPreviewCombatPlayer(player.id) : undefined}
      className={`phase-track phase-track--sequence${kind === 'step' ? ' phase-track--combat' : ''}`}
      style={concealed ? { visibility: 'hidden', pointerEvents: 'none' } : undefined}
      aria-hidden={concealed || undefined} inert={concealed || undefined}
      data-compact={compact ? 'true' : 'false'} aria-label={`${state?.players?.length ? `${playerName(player)}: ` : ''}${kind === 'step' ? ui('Combat steps') : t('game.phaseTrack', null, 'Turn phases')}`}>
      <ol className="phase-track-steps">
        {entries.map((entry, index) => {
          const key = `${kind}:${entry}`;
          const mode = priorityStopMode(priorityStops, key, player.id);
          const currentEntry = ownsTurn(player) && entry === current;
          return <li key={key} className="phase-track-cell" data-phase-name={entry}
            data-phase-active={currentEntry ? 'true' : 'false'}
            data-phase-complete={ownsTurn(player) && entries.indexOf(current) > index ? 'true' : 'false'}
            data-stop={mode || 'off'} aria-current={currentEntry ? 'step' : undefined}>
            {renderStop(entry, kind, player)}
            {entry === 'Combat' && kind === 'phase' && Number(player.id) === Number(self.id) && <span className="phase-track-empty-combat">
              {['attackers', 'blockers'].map(combatKind => {
                const attacking = combatKind === 'attackers';
                const enabled = (attacking ? priorityStatus.stopWithAttackers : priorityStatus.stopWithBlockers) !== false;
                const Icon = attacking ? Swords : Shield;
                const description = `When on, stop to declare ${combatKind} whenever you have legal ${combatKind}, even with no combat stops set. When off, normal phase stop settings apply.${attacking ? ' With no combat stops set, declare no attackers automatically (required attacks still stop).' : ''}`;
                return <button key={combatKind} type="button" className="phase-track-empty-combat-toggle"
                  data-combat-skip={combatKind} aria-label={`Stop with legal ${combatKind}`}
                  aria-pressed={enabled} disabled={!state} title={`${attacking ? 'Attackers' : 'Blockers'}: ${enabled ? 'On' : 'Off'}. ${description}`}
                  onClick={() => { if (!locked) priorityPolicy?.setCombatParticipationStop(combatKind, !enabled, state); }}>
                  <Icon size={12} aria-hidden="true" />
                </button>;
              })}
            </span>}
          </li>;
        })}
      </ol>
    </nav>
  );
  const renderPlayer = (player, local = false) => <section key={player.id}
    onMouseEnter={() => setPreviewCombatPlayer(player.id)}
    className={`phase-track-player${local ? ' phase-track-player--self' : ''}`}
    data-phase-player={player.id} data-active-turn={ownsTurn(player)} data-has-priority={hasPriority(player)}>
    {renderHeading(player)}
    {renderTrack(PHASE_TRACK, active, 'phase', player)}
    {duel && (active === 'Combat' || previewCombatPlayer === player.id || reserveCombatSpace) && renderTrack(COMBAT_STEPS, priorityStepKey(state), 'step', player, active !== 'Combat' && previewCombatPlayer !== player.id)}
  </section>;
  const combatOwner = players.find(player => player.id === previewCombatPlayer) || players.find(ownsTurn) || self;

  return <div onMouseLeave={() => setPreviewCombatPlayer(null)}
    onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) setPreviewCombatPlayer(null); }}
    className={`phase-track-group phase-track-group--players${multiplayer ? ' phase-track-group--fields' : ''}${duel ? ' phase-track-group--duel' : ''}`}>
    <button type="button" className="phase-track-lock" aria-pressed={locked} onClick={() => setLocked(!locked)} title={ui('Lock phase stops against accidental clicks')}>{locked ? ui('Unlock stops') : ui('Lock stops')}</button>
    {duel ? <div className="phase-track-player-grid">{turnPlayers.map(player => renderPlayer(player, player.id === self.id))}</div> : <div className="phase-track-player-grid">
      {multiplayer && <div className="phase-track-opponents" style={{ '--phase-opponent-count': opponents.length }}>
        {opponents.map(player => renderPlayer(player))}
      </div>}
      {renderPlayer(self, multiplayer)}
    </div>}
    {!duel && (active === 'Combat' || previewCombatPlayer !== null || reserveCombatSpace) && <div className={multiplayer ? 'phase-track-local-combat' : undefined}>
      {renderTrack(COMBAT_STEPS, priorityStepKey(state), 'step', combatOwner, active !== 'Combat' && previewCombatPlayer === null)}
    </div>}
  </div>;
}
