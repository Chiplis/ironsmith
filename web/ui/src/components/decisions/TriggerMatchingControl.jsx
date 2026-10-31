import { useGame } from '@/context/GameContext';
import usePrioritySettings from '@/hooks/usePrioritySettings';
import { Button } from '@/components/ui/button';
import PriorityShortcutHint from './PriorityShortcutHint';

export default function TriggerMatchingControl({ canAct }) {
  const { state, priorityPolicy } = useGame();
  const { groupTriggers } = usePrioritySettings();
  if (state?.decision?.choice_kind !== 'trigger_order' || !priorityPolicy) return null;
  return (
    <Button
      variant="outline"
      size="sm"
      className="shrink-0 self-center"
      aria-label="Automatically stack matching triggers"
      aria-pressed={!!groupTriggers}
      disabled={!canAct || Number(state.decision.player) !== Number(state.perspective)}
      title="Automatically stack matching target-free triggers. This setting lasts for the game; click or press 7 to toggle."
      onClick={() => priorityPolicy.setGroupTriggers(!priorityPolicy.status(state).groupTriggers, state)}
    >
      Trigger matching: {groupTriggers ? 'On' : 'Off'} <PriorityShortcutHint number="7" />
    </Button>
  );
}
