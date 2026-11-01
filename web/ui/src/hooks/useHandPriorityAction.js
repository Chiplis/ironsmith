import { useHandOpen, useSelectedHandCardId } from '@/lib/hand-open';
import { handPriorityAction } from '@/lib/hand-priority-action';

export default function useHandPriorityAction(state) {
  return handPriorityAction(state, useHandOpen(), useSelectedHandCardId());
}
