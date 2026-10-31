import { useSyncExternalStore } from 'react';
import { useHandOpen } from '@/lib/hand-open';
import { readShortcutPreset, subscribeShortcutPreset } from '@/lib/keyboard-preferences';

export default function PriorityShortcutHint({ number }) {
  const preset = useSyncExternalStore(subscribeShortcutPreset, readShortcutPreset, () => 'mtgo');
  const handOpen = useHandOpen();
  if (number == null || (preset === 'mtgo' && handOpen)) return null;
  const shortcut = preset === 'classic' ? `Alt+${number}` : String(number);
  return <kbd className="priority-shortcut-hint" aria-hidden="true" title={`Keyboard shortcut: ${shortcut}`}>{shortcut}</kbd>;
}
