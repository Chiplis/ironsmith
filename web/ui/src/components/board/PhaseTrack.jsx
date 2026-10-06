import { CircleCheck, FlagTriangleRight, Hand, Shield, Sword, Swords, Zap } from 'lucide-react';
import { useGame } from '@/context/GameContext';
import { useI18n } from '@/i18n/I18nContext';
import { normalizePhaseStep, PHASE_TRACK } from '@/lib/constants';
import { COMBAT_STEPS, priorityStepKey } from '@/lib/priority-stops';

const combatLabels = {
  BeginCombat: 'Beginning', DeclareAttackers: 'Attackers', DeclareBlockers: 'Blockers',
  FirstStrikeDamage: 'First strike', CombatDamage: 'Damage', EndCombat: 'End',
};

const combatIcons = {
  BeginCombat: FlagTriangleRight, DeclareAttackers: Swords, DeclareBlockers: Shield,
  FirstStrikeDamage: Zap, CombatDamage: Sword, EndCombat: CircleCheck,
};

export default function PhaseTrack({ compact = false }) {
  const { state, priorityStops = {}, cyclePriorityStop } = useGame();
  const { t } = useI18n();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const renderTrack = (entries, current, kind) => (
    <nav className={`phase-track phase-track--sequence${kind === 'step' ? ' phase-track--combat' : ''}`}
      data-compact={compact ? 'true' : 'false'} aria-label={kind === 'step' ? 'Combat steps' : t('game.phaseTrack', null, 'Turn phases')}>
      <ol className="phase-track-steps">
        {entries.map((entry, index) => {
          const key = `${kind}:${entry}`;
          const Icon = kind === 'step' ? combatIcons[entry] : Hand;
          const mode = priorityStops[key];
          const disabled = !state || entry === 'Untap';
          const label = kind === 'step' ? combatLabels[entry] : t(`game.track.${entry}`, null, entry);
          const status = mode === 'always' ? 'Stop every time' : mode === 'once' ? 'Stop once' : 'No stop';
          return (
            <li key={key} className="phase-track-cell" data-phase-name={entry}
              data-phase-active={entry === current ? 'true' : 'false'}
              data-phase-complete={entries.indexOf(current) > index ? 'true' : 'false'}
              data-stop={mode || 'off'} aria-current={entry === current ? 'step' : undefined}>
              <button type="button" className="phase-track-stop-button" disabled={disabled}
                aria-label={`${label}: ${disabled ? 'No priority during untap' : status}`}
                aria-pressed={Boolean(mode)} title={disabled ? 'No priority during untap' : `${label}: ${status}. Click to cycle once, every time, off.`}
                onClick={() => cyclePriorityStop?.(key)}>
                <span className="phase-track-step-marker" aria-hidden="true">{index + 1}</span>
                {!disabled && <Icon className="phase-track-stop-icon" size={14} aria-hidden="true" />}
                <span className="phase-track-step-label">{label}</span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
  return (
    <div className="phase-track-group">
      {active === 'Combat' && renderTrack(COMBAT_STEPS, priorityStepKey(state), 'step')}
      {renderTrack(PHASE_TRACK, active, 'phase')}
    </div>
  );
}
