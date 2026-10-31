import usePrioritySettings from "@/hooks/usePrioritySettings";
import { Hand, Settings2, Pause, X } from 'lucide-react';
import { useGame } from '@/context/GameContext';
import { yieldDescription } from '@/lib/priority-policy.js';
export default function PriorityHoldControl({ compact = false, battlefield = false }) {
  const { state, priorityPolicy, clearPriorityYields } = useGame();
  const priorityStatus = usePrioritySettings();
  if (compact) return null;
  const status = priorityStatus || { fullControl: false };
  return <>
    <button type="button" className="player-full-control" aria-pressed={status.fullControl}
      aria-label="Full control" title="Full control: confirm every priority and choice"
      onClick={event => { event.stopPropagation(); priorityPolicy?.setFullControl(!status.fullControl, state); }}>
      <Hand size={14}/><span className={battlefield ? 'sr-only' : undefined}>Full control</span>
    </button>
    <button type="button" className={battlefield ? 'battlefield-auto-pass-toggle' : 'player-priority-hold'}
      aria-label="Priority and yields" title="Priority stops, temporary yields and keyboard shortcuts"
      onClick={event => { event.stopPropagation(); window.dispatchEvent(new CustomEvent('ironsmith:priority-menu')); }}>
      <Settings2 size={18}/><span className={battlefield ? 'sr-only' : undefined}>Priority</span>
    </button>
    {status.holdNextAction && <button type="button" className="priority-control-status"
      aria-label="Hold next priority ×" title="Holding priority after your next spell or ability. Click to cancel."
      onClick={() => priorityPolicy?.holdNextAction(false, state)}>{battlefield && <Pause size={14}/>}
      <span className={battlefield ? 'sr-only' : undefined}>Hold next priority ×</span></button>}
    {status.resolveGroup && <button type="button" className="priority-control-status" onClick={clearPriorityYields}
      title="Resolve group active — repeat only identical choices. Click to cancel." aria-label="Cancel resolve group">
      <span className={battlefield ? 'sr-only' : undefined}>Resolve group ×</span>{battlefield && <X size={14}/>}</button>}
    {status.yield && <button type="button" className="priority-control-status" onClick={clearPriorityYields}
      title={`${yieldDescription(status.yield, state)}. Click to clear yields`} aria-label="Cancel active yield">
      {battlefield && <X size={14}/>}<span className={battlefield ? 'sr-only' : undefined}>{yieldDescription(status.yield, state)} ×</span></button>}
  </>;
}
