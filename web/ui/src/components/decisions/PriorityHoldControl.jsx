import usePrioritySettings from "@/hooks/usePrioritySettings";
import useUiText from '@/i18n/useUiText';
import { Hand, Settings2, Pause, X } from 'lucide-react';
import { useGame } from '@/context/GameContext';
import { yieldDescription, STACK_PRIORITY_MODES, STACK_PRIORITY_LABELS } from '@/lib/priority-policy.js';
export default function PriorityHoldControl({ compact = false, battlefield = false }) {
  const { state, priorityPolicy, clearPriorityYields } = useGame();
  const priorityStatus = usePrioritySettings();
  const ui = useUiText();
  if (compact) return null;
  const status = priorityStatus || { fullControl: false };
  const mode = status.stackMode || 'hold';
  return <>
    <button type="button" className="priority-stack-mode" aria-label={ui('Stack priority: {0}', { 0: ui(STACK_PRIORITY_LABELS[mode]) })}
      title={ui('Stack priority: {0}. H: Hold; C: Countdown (two seconds); P: Pass immediately. Click to change.', { 0: ui(STACK_PRIORITY_LABELS[mode]) })}
      onClick={event => {
        event.stopPropagation();
        priorityPolicy?.setStackMode(STACK_PRIORITY_MODES[(STACK_PRIORITY_MODES.indexOf(mode) + 1) % 3], state);
      }}><span aria-hidden="true">{ui(STACK_PRIORITY_LABELS[mode])[0]}</span></button>
    <button type="button" className="player-full-control" aria-pressed={status.fullControl}
      aria-label={ui('Full control')} title={ui('Full control: confirm every priority and choice')}
      onClick={event => { event.stopPropagation(); priorityPolicy?.setFullControl(!status.fullControl, state); }}>
      <Hand size={14}/><span className={battlefield ? 'sr-only' : undefined}>{ui('Full control')}</span>
    </button>
    <button type="button" className={battlefield ? 'battlefield-auto-pass-toggle' : 'player-priority-hold'}
      aria-label={ui('Priority and yields')} title={ui('Priority stops, temporary yields and keyboard shortcuts')}
      onClick={event => { event.stopPropagation(); window.dispatchEvent(new CustomEvent('ironsmith:priority-menu')); }}>
      <Settings2 size={18}/><span className={battlefield ? 'sr-only' : undefined}>{ui('Priority')}</span>
    </button>
    {status.holdNextAction && <button type="button" className="priority-control-status"
      aria-label={ui('Hold next priority ×')} title={ui('Holding priority after your next spell or ability. Click to cancel.')}
      onClick={() => priorityPolicy?.holdNextAction(false, state)}>{battlefield && <Pause size={14}/>}
      <span className={battlefield ? 'sr-only' : undefined}>{ui('Hold next priority ×')}</span></button>}
    {status.resolveGroup && <button type="button" className="priority-control-status" onClick={clearPriorityYields}
      title={ui('Resolve group active — repeat only identical choices. Click to cancel.')} aria-label={ui('Cancel resolve group')}>
      <span className={battlefield ? 'sr-only' : undefined}>{ui('Resolve group ×')}</span>{battlefield && <X size={14}/>}</button>}
    {status.yield && <button type="button" className="priority-control-status" onClick={clearPriorityYields}
      title={ui('{0}. Click to clear yields', { 0: yieldDescription(status.yield, state, ui) })} aria-label={ui('Cancel active yield')}>
      {battlefield && <X size={14}/>}<span className={battlefield ? 'sr-only' : undefined}>{yieldDescription(status.yield, state, ui)} ×</span></button>}
  </>;
}
