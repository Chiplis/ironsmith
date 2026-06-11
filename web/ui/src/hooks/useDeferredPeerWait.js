import { useEffect, useState } from "react";

// Most multiplayer action round-trips settle well under this; swapping the
// decision buttons to spinner content for every one of them reads as flicker.
const PEER_WAIT_SHOW_DELAY_MS = 280;

// Returns the peer wait only once it has been pending long enough to be worth
// showing. Use the returned value for VISUALS (button content, popover); keep
// gating interactions on the raw peer wait so in-flight actions stay locked.
export default function useDeferredPeerWait(peerWait, showDelayMs = PEER_WAIT_SHOW_DELAY_MS) {
  const active = Boolean(peerWait);
  const [visible, setVisible] = useState(active);

  // Hide immediately when the wait clears (state adjustment during render).
  if (!active && visible) {
    setVisible(false);
  }

  useEffect(() => {
    if (!active || visible) return undefined;
    const timer = setTimeout(() => setVisible(true), showDelayMs);
    return () => clearTimeout(timer);
  }, [active, visible, showDelayMs]);

  return active && visible ? peerWait : null;
}
