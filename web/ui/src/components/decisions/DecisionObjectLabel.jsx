import { useMemo } from 'react';
import { useGame } from '@/context/GameContext';
import ZoneIcon from '@/components/board/ZoneIcon';
import { buildObjectLocationById, decisionObjectLocation } from '@/lib/decision-object-meta';
import { getPlayerAccent } from '@/lib/player-colors';
import useUiText from '@/i18n/useUiText';

export default function DecisionObjectLabel({ objectId, object, children, nameOnly = false }) {
  const { state, playerAccentOverrides } = useGame();
  const ui = useUiText();
  const locations = useMemo(() => buildObjectLocationById(state), [state]);
  const location = decisionObjectLocation(locations, objectId, object);
  if (!location) return children;
  const playerName = id => state?.players?.find(player => String(player.id) === String(id))?.name || `Player ${id}`;
  const color = id => id == null ? undefined : getPlayerAccent(state?.players, id, state?.perspective, playerAccentOverrides)?.hex;
  const differentController = location.owner != null && location.controller != null
    && String(location.owner) !== String(location.controller);
  const zoneName = ui(location.zone[0].toUpperCase() + location.zone.slice(1));
  const description = [zoneName,
    location.owner != null && ui('Owner: {0}', { 0: playerName(location.owner) }),
    differentController && ui('Controller: {0}', { 0: playerName(location.controller) }),
  ].filter(Boolean).join(' · ');
  return <span className="decision-object-label" style={{ '--decision-object-controller': differentController ? color(location.controller) : undefined }}>
    <span className={`decision-object-label-text${nameOnly ? ' decision-object-name' : ''}`}>{children}</span>
    <span className="decision-object-zone" style={{ color: color(location.owner) }} title={description} role="img" aria-label={description}>
      <ZoneIcon zone={location.zone} />
    </span>
  </span>;
}
