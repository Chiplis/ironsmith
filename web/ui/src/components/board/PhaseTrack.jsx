import usePrioritySettings from "@/hooks/usePrioritySettings";
import { useState } from "react";
import { CircleCheck, FlagTriangleRight, Hand, Shield, Sword, Swords, Zap } from 'lucide-react';
import { useGame } from '@/context/GameContext';
import { useI18n } from '@/i18n/I18nContext';
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

// Lucide's hand is open strokes, so a fill leaves gaps between them. The
// closed silhouette underneath carries the stop color across the whole hand.
const HAND_OUTLINE = [
  'M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2',
  'M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2',
  'M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8',
  'M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15',
];
const HAND_SILHOUETTE = 'M6 14V6a2 2 0 0 1 4 0V4a2 2 0 0 1 4 0v2a2 2 0 0 1 4 0v2a2 2 0 0 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82Z';
function StopHand({ className, size = 24 }) {
  return <svg xmlns="http://www.w3.org/2000/svg" width={size} height={size} viewBox="0 0 24 24" fill="none"
    stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={`${className || ''} phase-track-stop-hand`} aria-hidden="true">
    <path className="phase-track-stop-fill" d={HAND_SILHOUETTE} />
    {HAND_OUTLINE.map(d => <path key={d} d={d} />)}
  </svg>;
}

export default function PhaseTrack({ compact = false, reserveCombatSpace = false }) {
  const { state, priorityStops = DEFAULT_PRIORITY_STOPS, cyclePriorityStop, setPriorityStop } = useGame();
  const priorityStatus = usePrioritySettings();
  const [locked, setLocked] = useState(false);
  const { t } = useI18n();
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
  const phaseLabel = (entry, kind) => kind === 'step' ? combatLabels[entry] : t(`game.track.${entry}`, null, entry);

  const renderStop = (entry, kind, player, shared = false) => {
    const key = `${kind}:${entry}`;
    const Icon = kind === 'step' ? combatIcons[entry] : StopHand;
    const mode = priorityStopMode(priorityStops, key, player.id);
    const disabled = !state || entry === 'Untap';
    const label = phaseLabel(entry, kind);
    const status = mode === 'always' ? 'Stop every time' : mode ? 'Stop when I can act' : 'No stop';
    return <button type="button" className="phase-track-stop-button" disabled={disabled}
      data-stop-player={player.id} data-stop-phase={entry} data-stop={mode || 'off'}
      data-yield-target={priorityStatus?.yield?.mode === 'until' && priorityStatus.yield.key === key
        && Number(priorityStatus.yield.player) === Number(player.id) ? 'true' : undefined}
      data-opponent-stop={shared && Number(player.id) !== Number(self.id) ? 'true' : undefined}
      aria-label={`${shared ? `${playerName(player)}: ` : ''}${label}: ${disabled ? 'No priority during untap' : status}`}
      aria-pressed={Boolean(mode)} title={`${playerName(player)} — ${label}: ${disabled ? 'No priority during untap' : `${status}. Click to toggle. Right-click for future yields and advanced stops.`}`}
      onContextMenu={event => {
        event.preventDefault();
        window.dispatchEvent(new CustomEvent('ironsmith:priority-menu', { detail: { phase: key, player: player.id } }));
      }}
      onClick={() => { if (!locked) {
        if (setPriorityStop) setPriorityStop(key, mode ? "off" : "always", player.id);
        else cyclePriorityStop?.(key, state?.players?.length ? player.id : undefined);
      } }}>
      {!disabled && <Icon className="phase-track-stop-icon" size={14} aria-hidden="true" />}
      {!shared && <span className="phase-track-step-label">{label}</span>}
    </button>;
  };

  const renderHeading = player => <header className="phase-track-player-heading">
    <span className="phase-track-player-name">{playerName(player)}</span>
    {ownsTurn(player) && <span className="phase-track-turn-cue">Turn</span>}
    {hasPriority(player) && <span className="phase-track-priority-cue">Priority</span>}
  </header>;

  const renderDuelLegend = () => <div className="phase-track-duel-legend">
    {turnPlayers.map(player => <div key={player.id} data-active-turn={ownsTurn(player)} data-has-priority={hasPriority(player)}>
      <Hand size={12} aria-hidden="true" className={player.id === self.id ? undefined : 'phase-track-opponent-hand'} />
      {renderHeading(player)}
    </div>)}
  </div>;

  const renderTrack = (entries, current, kind, player, concealed = false, shared = false) => (
    <nav className={`phase-track phase-track--sequence${kind === 'step' ? ' phase-track--combat' : ''}${shared ? ' phase-track--shared' : ''}`}
      style={concealed ? { visibility: 'hidden', pointerEvents: 'none' } : undefined}
      aria-hidden={concealed || undefined} inert={concealed || undefined}
      data-compact={compact ? 'true' : 'false'} aria-label={`${!shared && state?.players?.length ? `${playerName(player)}: ` : ''}${kind === 'step' ? 'Combat steps' : t('game.phaseTrack', null, 'Turn phases')}`}>
      <ol className="phase-track-steps">
        {entries.map((entry, index) => {
          const key = `${kind}:${entry}`;
          const mode = priorityStopMode(priorityStops, key, player.id);
          const currentEntry = (shared || ownsTurn(player)) && entry === current;
          return <li key={key} className="phase-track-cell" data-phase-name={entry}
            data-phase-active={currentEntry ? 'true' : 'false'}
            data-phase-complete={(shared || ownsTurn(player)) && entries.indexOf(current) > index ? 'true' : 'false'}
            data-stop={shared ? undefined : mode || 'off'} aria-current={currentEntry ? 'step' : undefined}>
            {shared ? <>
              {entry === 'Untap' && kind === 'phase' ? renderDuelLegend() : <div className="phase-track-shared-stops">
                {turnPlayers.map(owner => <div key={owner.id} className="phase-track-player-stop"
                  data-phase-player={owner.id} data-active-turn={ownsTurn(owner)} data-has-priority={hasPriority(owner)}>
                  {renderStop(entry, kind, owner, true)}
                </div>)}
              </div>}
              <span className="phase-track-step-label">{phaseLabel(entry, kind)}</span>
            </> : renderStop(entry, kind, player)}
          </li>;
        })}
      </ol>
    </nav>
  );
  const renderPlayer = (player, local = false) => <section key={player.id}
    className={`phase-track-player${local ? ' phase-track-player--self' : ''}`}
    data-phase-player={player.id} data-active-turn={ownsTurn(player)} data-has-priority={hasPriority(player)}>
    {renderHeading(player)}
    {renderTrack(PHASE_TRACK, active, 'phase', player)}
  </section>;
  const combatOwner = players.find(ownsTurn) || self;

  return <div className={`phase-track-group phase-track-group--players${multiplayer ? ' phase-track-group--fields' : ''}${duel ? ' phase-track-group--duel' : ''}`}>
    <button type="button" className="phase-track-lock" aria-pressed={locked} onClick={() => setLocked(!locked)} title="Lock phase stops against accidental clicks">{locked ? "Unlock stops" : "Lock stops"}</button>
    {duel ? <>
      {renderTrack(PHASE_TRACK, active, 'phase', self, false, true)}
    </> : <div className="phase-track-player-grid">
      {multiplayer && <div className="phase-track-opponents" style={{ '--phase-opponent-count': opponents.length }}>
        {opponents.map(player => renderPlayer(player))}
      </div>}
      {renderPlayer(self, multiplayer)}
    </div>}
    {(active === 'Combat' || reserveCombatSpace) && <div className={multiplayer ? 'phase-track-local-combat' : undefined}>
      {renderTrack(COMBAT_STEPS, priorityStepKey(state), 'step', combatOwner, active !== 'Combat', duel)}
    </div>}
  </div>;
}
