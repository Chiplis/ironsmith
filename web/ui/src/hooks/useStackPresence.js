import { useEffect, useRef, useState } from "react";
import { stackEntryRenderKeys } from "@/lib/stack-targets";

export const STACK_EXIT_MS = 360;

function mergeEntries(live, leaving) {
  const entries = [...live];
  for (const entry of leaving) {
    entries.splice(Math.min(entry.__previousIndex, entries.length), 0, entry);
  }
  return entries;
}

// Keep removed occurrences mounted for their exit motion, including the last
// card. Bottom-counted keys distinguish abilities sharing a snapshot id.
export default function useStackPresence(objects) {
  const keys = stackEntryRenderKeys(objects);
  // Compare snapshot contents: resolving entries can arrive as a fresh array
  // on every render. Save text changes too, so an exit uses the latest card.
  const signature = JSON.stringify(objects);
  const live = objects.map((entry, index) => ({
    ...entry,
    __timeline_key: `live-${keys[index]}`,
    __leaving: false,
  }));
  const timersRef = useRef(new Map());
  const [presence, setPresence] = useState({ signature, live, leaving: [] });
  let current = presence;
  if (presence.signature !== signature) {
    const liveKeys = new Set(live.map(entry => entry.__timeline_key));
    const leaving = mergeEntries(presence.live, presence.leaving)
      .map((entry, index) => entry.__leaving ? entry : ({
        ...entry,
        __leaving: true,
        __previousIndex: index,
      }))
      .filter(entry => !liveKeys.has(entry.__timeline_key));
    current = { signature, live, leaving };
    setPresence(current);
  }
  const entries = mergeEntries(live, current.leaving);
  useEffect(() => {
    const timers = timersRef.current;
    for (const [entry, timer] of timers) {
      if (presence.leaving.includes(entry)) continue;
      window.clearTimeout(timer);
      timers.delete(entry);
    }
    for (const entry of presence.leaving) {
      if (timers.has(entry)) continue;
      timers.set(entry, window.setTimeout(() => {
        timers.delete(entry);
        setPresence(previous => ({
          ...previous,
          leaving: previous.leaving.filter(candidate => candidate !== entry),
        }));
      }, STACK_EXIT_MS));
    }
  }, [presence]);
  useEffect(() => {
    const timers = timersRef.current;
    return () => {
      for (const timer of timers.values()) window.clearTimeout(timer);
      timers.clear();
    };
  }, []);
  return entries;
}
