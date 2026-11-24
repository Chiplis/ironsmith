import { useEffect, useRef } from "react";
import { createTrustedSequencer } from "../../lib/verified-session/trusted-sequencer.js";
import { browserSessionHost } from "./browser-host.js";

// React adapter: the logic lives in lib/verified-session/trusted-sequencer.js.
export function useTrustedSequencer(base, servicesRef) {
  const instance = useRef(null);
  if (!instance.current) instance.current = createTrustedSequencer(base, servicesRef, browserSessionHost);
  // Mutable session/service refs are intentional; never restart retry timers on clock ticks.
  useEffect(() => {
    instance.current.start();
    return () => instance.current.stop();
  }, []);
  return instance.current.api;
}
