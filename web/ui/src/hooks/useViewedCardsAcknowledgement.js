import { useEffect } from "react";
import { LOOK_DONE_EVENT } from "@/lib/look-pile";

// The engine records successful prompts that displayed the whole card view.
// Keep visibility intact, but dismiss its temporary Look pile on returning to priority.
export default function useViewedCardsAcknowledgement(decision, viewedCards, identity) {
  const acknowledged = viewedCards?.acknowledged === true;
  const completed = acknowledged && decision?.kind === "priority";
  useEffect(() => {
    if (completed) window.dispatchEvent(new Event(LOOK_DONE_EVENT));
  }, [completed, identity]);
  return acknowledged;
}
