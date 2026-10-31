import { useContext, useSyncExternalStore } from 'react';
import { GameContext } from '@/context/GameContext.shared';
const idleSubscribe = () => () => {};
const idleSnapshot = () => 0;
const empty = { fullControl: false, holdNextAction: false, yield: null, rules: [], groupTriggers: false };
// Preference edits should repaint the controls, not every card on the table.
export default function usePrioritySettings() {
  const context = useContext(GameContext);
  const policy = context?.priorityPolicy;
  useSyncExternalStore(policy?.subscribe || idleSubscribe, policy?.getSnapshot || idleSnapshot);
  return policy?.status(context.state) || context?.priorityStatus || empty;
}
