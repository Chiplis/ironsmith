import { useGame } from '@/context/GameContext';
import { activateSelectedHandCard } from '@/lib/hand-priority-action';
import useUiText from '@/i18n/useUiText';
import { Button } from '@/components/ui/button';

export default function HandPriorityAction({ action, disabled, className = '', style }) {
  const ui = useUiText();
  const { multiplayer } = useGame();
  const blocked = disabled || !action.actions.length || multiplayer?.submittingAction;
  return <Button variant="ghost" size="sm" className={`decision-neon-button decision-main-button hand-priority-action flex-col gap-1 ${className}`}
    style={style} disabled={blocked} aria-label={ui(action.label)} data-hand-primary-action="true"
    onPointerDown={event => event.preventDefault()}
    onClick={() => activateSelectedHandCard(action.card?.id)}>
    <span>{ui(action.label)}</span>
    {action.actions.length > 0 && <kbd className="priority-shortcut-hint">{ui('Space')}</kbd>}
  </Button>;
}
