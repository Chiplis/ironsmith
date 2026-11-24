// Browser/React bindings for the framework-free session modules in
// lib/verified-session. Everything platform-specific the extracted modules need
// is listed here, so a different host (Manabrew) supplies its own object of the
// same shape instead of importing shared.js.
import { recordDiagnosticEvent } from "../../lib/action-diagnostics.js";
import { emitSyncFailureNotice, mergeAuditOpenings, recordPeerSyncPerf, safeSend } from "./shared.js";

export const browserSessionHost = Object.freeze({
  send: safeSend,
  recordDiagnosticEvent,
  recordPerf: recordPeerSyncPerf,
  notice: emitSyncFailureNotice,
  mergeAuditOpenings,
});
