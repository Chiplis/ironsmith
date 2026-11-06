import PriorityShortcutHint from "@/components/decisions/PriorityShortcutHint";
import usePrioritySettings from "@/hooks/usePrioritySettings";
import { useEffect, useState } from 'react';
import useUiText from '@/i18n/useUiText';
import { X } from 'lucide-react';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription, SheetClose, SheetFooter } from '@/components/ui/sheet';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { useGame } from '@/context/GameContext';
import { abilityKey, yieldDescription, stopLabel, STACK_PRIORITY_MODES, STACK_PRIORITY_LABELS } from '@/lib/priority-policy.js';
import { readShortcutPreset, setShortcutPreset } from '@/lib/keyboard-preferences.js';
import { PRIORITY_STOP_KEYS, priorityStopMode } from '@/lib/priority-stops.js';
import './priority-preferences.css';

export default function PriorityPreferences() {
  const { state, priorityPolicy, startPriorityYield, clearPriorityYields,
    priorityStops = {}, cyclePriorityStop } = useGame();
  const priorityStatus = usePrioritySettings();
  const ui = useUiText();
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
        <div className="priority-preferences-eyebrow">{ui('Play controls')}</div>
        <SheetTitle>{entry ? entry.name : ui('Priority and yields')}</SheetTitle>
        <SheetDescription>{entry ? ui('Automatic responses for this ability.') : ui('Set your stops, yields and keyboard controls.')}</SheetDescription>
        <SheetClose asChild><Button variant="ghost" size="icon" className="priority-preferences-close" aria-label={ui('Close priority controls')}><X /></Button></SheetClose>
      </SheetHeader>
      <div className="priority-preferences-body">
    {menu.abilities ? <div className="priority-options">
      {menu.abilities.map(ability => <Button key={ability.automation_identity} variant="ghost"
        onClick={() => setMenu({entry: ability})}>{ability.source_ability_text}</Button>)}
    </div> : entry ? <>
      <p className="priority-help">{entry.source_ability_text || entry.ability_text}</p>
      {!valid && <p>{ui('Automation becomes available when this ability has a structural identity.')}</p>}
      <div className="priority-options">
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { yield: false })}>{ui('Always wait for this ability')}</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { yield: 'countdown' })}>{ui('Countdown before passing this ability')}</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { yield: true }, true)}>{ui('Yield to this ability until end of turn')}</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { yield: true })}>{ui('Always yield to this ability')}</Button>
        <Button variant="ghost" disabled={!abilityKey(entry, 'name')} onClick={() => rule('name', { yield: true })}>{ui('Yield to this ability from cards named {0}', { 0: entry.name })}</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { answer: 'yes' })}>{ui('Always yes to this ability’s optional choices')}</Button>
        <Button variant="ghost" disabled={!valid} onClick={() => rule('source', { answer: 'no' })}>{ui('Always no to this ability’s optional choices')}</Button>
      </div><p className="priority-help">{ui('Yielding passes priority only. Yes/no preferences apply to this ability while it resolves. Targets, payments and other selections still require your input.')}</p>
    </> : <>
      <label className="priority-check">{ui('Stack priority')}
        <select aria-label={ui('Stack priority behavior')} value={status.stackMode || 'hold'}
          onChange={event => priorityPolicy.setStackMode(event.target.value, state)}>
          {STACK_PRIORITY_MODES.map(mode => <option key={mode} value={mode}>{ui(STACK_PRIORITY_LABELS[mode])}</option>)}
        </select>
      </label>
      <p className="priority-help">{ui('Hold waits for your response. Countdown gives you two seconds to hold or hover to pause. Pass yields immediately. Ability-specific preferences override this setting.')}</p>
      <div className="priority-current" role="status">{status.resolveGroup ? ui('Resolve group — repeating only identical choices')
        : status.fullControl ? ui('Full control — every priority stays manual')
        : status.holdNextAction ? ui('Priority will be held after your next spell or ability')
        : status.yield ? yieldDescription(status.yield, state, ui) : ui('Normal play — saved stops and opponent responses')}</div>
      <div className="priority-options">
        {menu.phase && <Button variant="ghost" onClick={() => execute(() => startPriorityYield('until', { key: menu.phase, player: menu.player }))}>{ui('Yield until {0} on {1}’s turn', { 0: menu.phase.split(':')[1], 1: (state.players || []).find(player => player.id === menu.player)?.name })}</Button>}
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('responses'))}><span>{ui('Pass until a response is needed')}</span><PriorityShortcutHint number="2" /><small>{ui('This turn. Stop for opponents’ spells and required choices.')}</small></Button>
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('turn'))}><span>{ui('Yield through this turn')}</span><PriorityShortcutHint number="6" /><small>{ui('Skip responses and declare no attackers. Required attacks, blockers and other choices stay manual.')}</small></Button>
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('no_play'))}><span>{ui('Yield when no play is possible')}</span><PriorityShortcutHint number="8" /><small>{ui('All turns this game. Pause whenever a legal play is available.')}</small></Button>
        <Button variant="ghost" onClick={() => execute(() => startPriorityYield('until', { key: 'phase:End', anyPlayer: true }))}>{ui('Yield until the next end step')}</Button>
      </div>
      <label className="priority-check"><Checkbox checked={!!status.holdNextAction}
        onCheckedChange={checked => priorityPolicy.holdNextAction(Boolean(checked), state)} />{ui('Hold priority after my next spell or ability')}</label>
      <label className="priority-check"><Checkbox checked={!!status.fullControl}
        onCheckedChange={checked => priorityPolicy.setFullControl(Boolean(checked), state)} />{ui('Full control: confirm every priority and choice')}</label>
      <label className="priority-check"><Checkbox checked={!!status.groupTriggers}
        onCheckedChange={checked => priorityPolicy.setGroupTriggers(Boolean(checked), state)} />{ui('Automatically order identical target-free triggers')} <PriorityShortcutHint number="7" /></label>
      <details open={!!menu.phase || undefined}><summary>{ui('Phase stops and future yields')}</summary><p className="priority-help">{ui('Each row applies on that player’s turn. Stops wait for you; they never count down.')}</p>
        {(state.players || []).map(player => <div key={player.id} className="priority-stop-list"><strong>{ui('{0}’s turn', { 0: player.name })}</strong>
          {PRIORITY_STOP_KEYS.filter(key => !key.endsWith('Untap')).map(key => <div key={key}>
            <span>{ui(stopLabel(key))}</span>
            <Button variant="ghost" onClick={() => cyclePriorityStop?.(key, player.id)}>{ui({always:'Always',can_act:'If I can act'}[priorityStopMode(priorityStops, key, player.id)] || 'Off')}</Button>
            <Button variant="ghost" onClick={() => execute(() => startPriorityYield('until', { key, player: player.id }))}>{ui('Yield until here')}</Button>
          </div>)}</div>)}
      </details>
      <label className="priority-check">{ui('Coin flip choice')}
        <select aria-label={ui('Remembered coin flip choice')} value={status.coinChoice || ''}
          onChange={event => priorityPolicy.setCoinChoice(event.target.value || null, state)}>
          <option value="">{ui('Ask every time')}</option><option value="Heads">{ui('Heads')}</option><option value="Tails">{ui('Tails')}</option>
        </select>
      </label>
      <label className="priority-check">{ui('Keyboard preset')} <select value={preset} onChange={event => {
        setPreset(event.target.value); setShortcutPreset(event.target.value);
      }}><option value="mtgo">{ui('MTGO numbers')}</option><option value="classic">{ui('Classic hand numbers / Alt + number for yields')}</option></select></label>
      <p className="priority-help">{ui('Hold Ctrl while starting a spell or ability to retain priority after it reaches the stack. You can also arm the next-action checkbox above. H and . both open or close your numbered hand. Arrow keys stay within the hand while it is open. While your hand is open, number keys select cards in either preset. Close it to use MTGO priority numbers.')}</p>
      {status.rules.length > 0 && <p>{ui(status.rules.length === 1 ? '{0} saved ability preference for this game.' : '{0} saved ability preferences for this game.', { 0: status.rules.length })}</p>}
    </>}
    </div>
    <SheetFooter className="priority-preferences-footer"><Button variant="ghost" className="priority-clear" onClick={() => execute(clearPriorityYields)}>{ui('Clear all yields and automatic choices')} <PriorityShortcutHint number="5" /></Button>
      <span>{ui('Phase stops are preserved.')}</span></SheetFooter>
    </SheetContent>
  </Sheet>;
}
