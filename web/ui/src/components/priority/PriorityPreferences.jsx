import PriorityShortcutHint from "@/components/decisions/PriorityShortcutHint";
import usePrioritySettings from "@/hooks/usePrioritySettings";
import { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription, SheetClose, SheetFooter } from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { useGame } from '@/context/GameContext';
import { abilityKey, yieldDescription } from '@/lib/priority-policy.js';
import { readShortcutPreset, setShortcutPreset } from '@/lib/keyboard-preferences.js';
import { PRIORITY_STOP_KEYS, priorityStopMode } from '@/lib/priority-stops.js';
import './priority-preferences.css';

export default function PriorityPreferences() {
  const { state, priorityPolicy, startPriorityYield, clearPriorityYields,
    priorityStops = {}, cyclePriorityStop } = useGame();
  const priorityStatus = usePrioritySettings();
  const [menu, setMenu] = useState(null);
  const [preset, setPreset] = useState(readShortcutPreset);
  useEffect(() => {
    const open = event => setMenu(event.detail || {});
    window.addEventListener('ironsmith:priority-menu', open);
    return () => { window.removeEventListener('ironsmith:priority-menu', open); };
  }, []);
  if (!menu || !priorityPolicy) return null;
  const entry = menu.entry;
  const valid = abilityKey(entry);
  const execute = fn => { fn(); setMenu(null); };
  const status = priorityStatus || priorityPolicy.status(state);
  const rule = (scope, values, end = false) => execute(() => priorityPolicy.setRule(entry, scope, values, state, end));
  return <Sheet open onOpenChange={open => { if (!open) setMenu(null); }}>
    <SheetContent side="center" className="fantasy-sheet priority-preferences" showCloseButton={false}
      style={{ zIndex: 30501 }} overlayStyle={{ zIndex: 30500 }}>
      <SheetHeader className="fantasy-sheet-header">
        <div className="priority-preferences-eyebrow">Play controls</div>
        <SheetTitle>{entry ? entry.name : 'Priority and yields'}</SheetTitle>
        <SheetDescription>{entry ? 'Automatic responses for this ability.' : 'Set your stops, yields and keyboard controls.'}</SheetDescription>
        <SheetClose asChild><Button variant="ghost" size="icon" className="priority-preferences-close" aria-label="Close priority controls"><X /></Button></SheetClose>
      </SheetHeader>
      <div className="priority-preferences-body">
    {entry ? <>
      <p className="priority-help">{entry.source_ability_text || entry.ability_text}</p>
      {!valid && <p>Automation becomes available when this ability has a structural identity.</p>}
      <div className="priority-options">
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { yield: true }, true)}>Yield to this ability until end of turn</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { yield: true })}>Always yield to this ability</Button>
        <Button variant="ghost" disabled={!abilityKey(entry, 'name')} onClick={() => rule('name', { yield: true })}>Yield to this ability from cards named {entry.name}</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { answer: 'yes' })}>Always yes to this ability’s optional choices</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { answer: 'no' })}>Always no to this ability’s optional choices</Button>
      </div><p className="priority-help">Yielding passes priority only. Yes/no preferences apply to this ability while it resolves. Targets, payments and other selections still require your input.</p>
    </> : <>
      <div className="priority-current" role="status">{status.resolveGroup ? 'Resolve group — repeating only identical choices'
        : status.fullControl ? 'Full control — every priority stays manual'
        : status.holdNextAction ? 'Priority will be held after your next spell or ability'
        : status.yield ? yieldDescription(status.yield, state) : 'Normal play — saved stops and opponent responses'}</div>
      <div className="priority-options">
        {menu.phase && <Button variant="ghost" onClick={() => execute(() => startPriorityYield('until', { key: menu.phase, player: menu.player }))}>Yield until {menu.phase.split(':')[1]} on {(state.players || []).find(player => player.id === menu.player)?.name}’s turn</Button>}
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('responses'))}><span>Pass until a response is needed</span><PriorityShortcutHint number="2" /><small>This turn. Stop for opponents’ spells and required choices.</small></Button>
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('turn'))}><span>Yield through this turn</span><PriorityShortcutHint number="6" /><small>Skip responses. Attackers, blockers and other choices stay manual.</small></Button>
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('no_play'))}><span>Yield when no play is possible</span><PriorityShortcutHint number="8" /><small>All turns this game. Pause whenever a legal play is available.</small></Button>
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('until', { key: 'phase:End', anyPlayer: true }))}>Yield until the next end step</Button>
      </div>
      <label className="priority-check"><Checkbox checked={!!status.holdNextAction}
        onCheckedChange={checked => priorityPolicy.holdNextAction(Boolean(checked), state)} />Hold priority after my next spell or ability</label>
      <label className="priority-check"><Checkbox checked={!!status.fullControl}
        onCheckedChange={checked => priorityPolicy.setFullControl(Boolean(checked), state)} />Full control: confirm every priority and choice</label>
      <label className="priority-check"><Checkbox checked={!!status.groupTriggers}
        onCheckedChange={checked => priorityPolicy.setGroupTriggers(Boolean(checked), state)} />Automatically order identical target-free triggers <PriorityShortcutHint number="7" /></label>
      <details open={!!menu.phase || undefined}><summary>Phase stops and future yields</summary><p className="priority-help">Each row applies on that player’s turn. Stops wait for you; they never count down.</p>
        {(state.players || []).map(player => <div key={player.id} className="priority-stop-list"><strong>{player.name}’s turn</strong>
          {PRIORITY_STOP_KEYS.filter(key => !key.endsWith('Untap')).map(key => <div key={key}>
            <span>{key.replace(/^(phase|step):/, '').replace(/([a-z])([A-Z])/g, '$1 $2')}</span>
            <Button variant="ghost" onClick={() => cyclePriorityStop?.(key, player.id)}>{{always:'Always',can_act:'If I can act'}[priorityStopMode(priorityStops, key, player.id)] || 'Off'}</Button>
            <Button variant="ghost" onClick={() => execute(() => startPriorityYield('until', { key, player: player.id }))}>Yield until here</Button>
          </div>)}</div>)}
      </details>
      <label className="priority-check">Keyboard preset <select value={preset} onChange={event => {
        setPreset(event.target.value); setShortcutPreset(event.target.value);
      }}><option value="mtgo">MTGO numbers</option><option value="classic">Classic hand numbers / Alt + number for yields</option></select></label>
      <p className="priority-help">Hold Ctrl while starting a spell or ability to retain priority after it reaches the stack. You can also arm the next-action checkbox above. H opens your numbered hand; . toggles your hand. While your hand is open, number keys select cards in either preset. Close it to use MTGO priority numbers.</p>
      {status.rules.length > 0 && <p>{status.rules.length} saved ability preference{status.rules.length === 1 ? '' : 's'} for this game.</p>}
    </>}
    </div>
    <SheetFooter className="priority-preferences-footer"><Button variant="ghost" className="priority-clear" onClick={() => execute(clearPriorityYields)}>Clear all yields and automatic choices <PriorityShortcutHint number="5" /></Button>
      <span>Phase stops are preserved.</span></SheetFooter>
    </SheetContent>
  </Sheet>;
}
