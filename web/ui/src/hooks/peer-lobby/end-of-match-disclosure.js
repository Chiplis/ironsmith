import { useEffect, useRef } from "react";
import { createEndOfMatchDisclosure } from "../../lib/verified-session/end-of-match-disclosure.js";
import { browserSessionHost } from "./browser-host.js";

// The protocol (constants, payloads, verdict rules) lives in
// lib/verified-session/end-of-match-disclosure.js; re-exported for existing importers.
export {
  DISCLOSURE_STATUS_CHEAT,
  DISCLOSURE_STATUS_MISSING,
  DISCLOSURE_STATUS_PENDING,
  DISCLOSURE_STATUS_VERIFIED,
  DISCLOSURE_STATUS_WITHHELD,
  END_OF_MATCH_DISCLOSURE_DOMAIN,
  END_OF_MATCH_DISCLOSURE_TIMEOUT_MS,
  disclosureDueForPlayer,
  endOfMatchDisclosurePayload,
  endOfMatchDisclosureTranscriptEntry,
  pendingDisclosureClaimCount,
} from "../../lib/verified-session/end-of-match-disclosure.js";

// React adapter around createEndOfMatchDisclosure.
export function usePeerLobbyEndOfMatchDisclosure(base, servicesRef) {
  const instance = useRef(null);
  if (!instance.current) instance.current = createEndOfMatchDisclosure(base, servicesRef, browserSessionHost);
  useEffect(() => () => instance.current.dispose(), []);
  return instance.current.api;
}
