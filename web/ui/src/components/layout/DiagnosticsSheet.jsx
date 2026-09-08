import { cloneElement, isValidElement, useEffect, useState, useSyncExternalStore } from "react";
import { Activity, Check, ClipboardCopy, Eraser } from "lucide-react";
import { useGame, useMatchClock } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { readEngineDiagnostics } from "@/lib/engine-diagnostics";
import { copyTextToClipboard } from "@/lib/clipboard";
import {
  diagnosticsVersion,
  exportDiagnostics,
  getDiagnosticsSnapshot,
  resetDiagnostics,
  subscribeDiagnostics,
} from "@/lib/action-diagnostics";
import { useI18n } from "@/i18n/I18nContext";
import { playerDisplayName } from "@/lib/player-display";

const defaultTriggerClassName = "stone-pill table-zone-action-button inline-flex items-center justify-center gap-1 rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110";

const ms = (value) => (value == null || !Number.isFinite(Number(value)) ? "—" : `${Math.round(Number(value))} ms`);
const seconds = (value) => (value == null || !Number.isFinite(Number(value)) ? "—" : `${(Number(value) / 1000).toFixed(1)} s`);
const age = (value) => (value == null ? "—" : Number(value) < 1000 ? ms(value) : seconds(value));

// Three-way health reading: green when the number is comfortably low, amber
// when it would already be felt, red when it explains a stuck click.
function tone(value, warn, bad) {
  if (value == null || !Number.isFinite(Number(value))) return "muted";
  if (Number(value) >= bad) return "bad";
  if (Number(value) >= warn) return "warn";
  return "good";
}

function Stat({ label, value, tone: toneName = "muted", hint }) {
  return (
    <div className="fantasy-sheet-stat diagnostics-stat" data-tone={toneName}>
      <div className="diagnostics-stat-label">{label}</div>
      <div className="diagnostics-stat-value">{value}</div>
      {hint ? <div className="diagnostics-stat-hint">{hint}</div> : null}
    </div>
  );
}

function StageChips({ trace, nowMs }) {
  const stages = trace.stages || [];
  const items = stages.map((stage) => ({ key: `${stage.name}-${stage.at}`, name: stage.name, ms: stage.sincePreviousMs }));
  if (!trace.done) {
    const last = stages.at(-1);
    items.push({ key: "waiting", name: "waiting…", ms: nowMs - (last ? last.at : trace.startedAt), live: true });
  }
  return (
    <div className="diagnostics-stages">
      {items.map((item) => (
        <span key={item.key} className="diagnostics-stage" data-live={item.live ? "true" : undefined} data-tone={tone(item.ms, 1000, 5000)}>
          <span className="diagnostics-stage-name">{item.name}</span>
          <span className="diagnostics-stage-ms">+{Math.round(item.ms)} ms</span>
        </span>
      ))}
    </div>
  );
}

// The verdict summarises the evidence a player would otherwise have to read
// off four panels. It only names the leg the numbers actually point at.
function verdict({ snapshot, peerWait, multiplayer }) {
  const current = snapshot.current;
  const stalled = snapshot.mainThread.worstStallMs >= 2000;
  if (stalled && snapshot.mainThread.worstStallAt != null && snapshot.at - snapshot.mainThread.worstStallAt < 15000) {
    return { tone: "bad", text: `This browser tab froze for ${seconds(snapshot.mainThread.worstStallMs)} a moment ago. That is local load, not the network.` };
  }
  if (snapshot.engine && snapshot.engine.queueWaitMs >= 2000) {
    return { tone: "bad", text: `Commands are queuing ${seconds(snapshot.engine.queueWaitMs)} behind other engine work in the game worker.` };
  }
  const quietPeers = snapshot.peers.filter((peer) => peer.state !== "closed" && peer.sinceReceivedMs != null && peer.sinceReceivedMs > 8000);
  if (quietPeers.length) {
    return { tone: "bad", text: `Nothing has arrived from ${quietPeers.map((peer) => peer.name || peer.peerId).join(", ")} for ${seconds(Math.max(...quietPeers.map((peer) => peer.sinceReceivedMs)))}. Heartbeats normally arrive every 3 s, so this is the connection.` };
  }
  if (current && peerWait && !peerWait.local) {
    const who = Array.isArray(peerWait.peers) && peerWait.peers.length ? peerWait.peers.map((peer) => peer?.name).filter(Boolean).join(", ") : peerWait.peerName || "a peer";
    return { tone: "warn", text: `Your action is waiting on ${who} (${peerWait.kind || "peer"}${peerWait.operation ? `: ${peerWait.operation}` : ""}). Their connection is alive, so they are busy computing or deciding.` };
  }
  if (current && peerWait?.local) {
    return { tone: "warn", text: `This browser is still ${peerWait.operation ? peerWait.operation.toLowerCase() : "preparing the action"} (${peerWait.kind}).` };
  }
  if (current) {
    return { tone: "warn", text: `An action has been in flight for ${seconds(snapshot.at - current.startedAt)} with no peer wait recorded.` };
  }
  const slowRtt = snapshot.peers.filter((peer) => peer.rttMs != null && peer.rttMs > 1000);
  if (slowRtt.length) {
    return { tone: "warn", text: `Round trips to ${slowRtt.map((peer) => peer.name || peer.peerId).join(", ")} exceed 1 s; actions will feel sluggish even though nothing is stuck.` };
  }
  if (multiplayer?.mode !== "idle" && !snapshot.peers.length) {
    return { tone: "muted", text: "No peer traffic has been observed yet." };
  }
  return { tone: "good", text: "Nothing is waiting. The browser, the engine worker and the peers all look responsive." };
}

export default function DiagnosticsSheet({ trigger, triggerClassName = defaultTriggerClassName }) {
  const { game, multiplayer, state } = useGame();
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [tick, setTick] = useState(0);
  const [copied, setCopied] = useState(false);
  useSyncExternalStore(subscribeDiagnostics, diagnosticsVersion, diagnosticsVersion);
  // Ages keep counting while the sheet is open even when nothing new arrives.
  useEffect(() => {
    if (!open) return undefined;
    const timer = window.setInterval(() => setTick((value) => value + 1), 1000);
    return () => window.clearInterval(timer);
  }, [open]);
  void tick;

  const snapshot = open ? getDiagnosticsSnapshot() : null;
  const peerWait = multiplayer?.peerWait || null;
  const players = state?.players || [];
  const lobbyPlayers = Array.isArray(multiplayer?.players) ? multiplayer.players : [];
  const clock = useMatchClock();

  const report = async () => {
    const engine = await readEngineDiagnostics(game);
    return exportDiagnostics({
      multiplayer: {
        mode: multiplayer?.mode, role: multiplayer?.role, lastAppliedSequence: multiplayer?.lastAppliedSequence,
        submittingAction: multiplayer?.submittingAction, peerWait, connectionWarnings: multiplayer?.connectionWarnings, matchClock: clock,
        players: lobbyPlayers,
      },
      game: { phase: state?.phase, step: state?.step, decision: state?.decision?.kind, priority_player: state?.priority_player, turn: state?.turn_number ?? state?.turn },
      engine,
    }, state);
  };
  const handleCopy = async () => {
    try {
      const payload = await report();
      await copyTextToClipboard(`${JSON.stringify(payload, null, 2)}\n`);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      setCopied(false);
    }
  };
  const handleDownload = async () => {
    const payload = await report();
    const url = URL.createObjectURL(new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = `ironsmith-diagnostics-${Date.now()}.json`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  };

  const triggerNode = isValidElement(trigger)
    ? cloneElement(trigger, { onClick: (event) => { trigger.props.onClick?.(event); setOpen(true); } })
    : (
      <button type="button" className={triggerClassName} onClick={() => setOpen(true)}>
        <Activity className="size-3.5" aria-hidden="true" />
        {t("action.diagnostics")}
      </button>
    );

  return (
    <>
      {triggerNode}
      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent side="center" className="diagnostics-sheet fantasy-sheet overflow-hidden p-0" style={{ width: "min(96vw, 820px)", maxWidth: "820px" }}>
          <SheetHeader className="fantasy-sheet-header diagnostics-header pr-12">
            <div className="verify-match-eyebrow">Latency</div>
            <SheetTitle className="verify-match-title">{t("action.diagnostics")}</SheetTitle>
          </SheetHeader>
          {snapshot ? (() => {
            const summary = verdict({ snapshot, peerWait, multiplayer });
            const current = snapshot.current;
            // Match-clock epochs are stamped with the monotonic clock, like snapshot.at.
            const clockEpochAge = clock?.startedAtMs != null ? snapshot.at - Number(clock.startedAtMs) : null;
            const activeClockPlayer = clock?.activePlayerIndex != null ? playerDisplayName(players, clock.activePlayerIndex) || `P${Number(clock.activePlayerIndex) + 1}` : "—";
            return (
              <div className="diagnostics-body">
                <div className="diagnostics-verdict" data-tone={summary.tone}>{summary.text}</div>

                <section className="fantasy-sheet-section diagnostics-section">
                  <h3 className="diagnostics-section-title">This browser</h3>
                  <div className="diagnostics-stat-grid">
                    <Stat label="Event-loop lag" value={ms(snapshot.mainThread.lagMs)} tone={tone(snapshot.mainThread.lagMs, 100, 1000)} hint="how late a 500 ms timer fires" />
                    <Stat label="Worst freeze, last 60 s" value={snapshot.mainThread.worstStallMs ? seconds(snapshot.mainThread.worstStallMs) : "none"} tone={tone(snapshot.mainThread.worstStallMs, 200, 2000)} hint={snapshot.mainThread.worstStallAt != null ? `${age(snapshot.at - snapshot.mainThread.worstStallAt)} ago` : undefined} />
                    <Stat label="Engine queue wait" value={snapshot.engine ? ms(snapshot.engine.queueWaitMs) : "—"} tone={snapshot.engine ? tone(snapshot.engine.queueWaitMs, 200, 2000) : "muted"} hint={snapshot.engine ? `last ${snapshot.engine.method || "call"}, ${age(snapshot.at - snapshot.engine.at)} ago` : "no engine call yet"} />
                    <Stat label="Engine compute" value={snapshot.engine ? ms(snapshot.engine.wasmCallMs) : "—"} tone={snapshot.engine ? tone(snapshot.engine.wasmCallMs, 500, 5000) : "muted"} hint="wasm time inside the worker" />
                  </div>
                </section>

                <section className="fantasy-sheet-section diagnostics-section">
                  <h3 className="diagnostics-section-title">Match</h3>
                  <div className="diagnostics-stat-grid">
                    <Stat label="In flight" value={current ? `${current.label} · ${seconds(snapshot.at - current.startedAt)}` : "nothing"} tone={current ? tone(snapshot.at - current.startedAt, 1000, 8000) : "good"} hint={peerWait ? `${peerWait.kind || "wait"}${peerWait.operation ? ` · ${peerWait.operation}` : ""}` : undefined} />
                    <Stat label="Last applied sequence" value={multiplayer?.lastAppliedSequence ?? "—"} hint={multiplayer?.submittingAction ? "submitting" : "idle"} />
                    <Stat label="Clock" value={clock?.enabled ? `${activeClockPlayer} · ${seconds(clock.remainingMs)}` : "off"} hint={clock?.enabled ? `epoch started ${age(clockEpochAge)} ago · seq ${clock.lastSequence ?? "—"}` : undefined} tone={clock?.enabled && clockEpochAge != null && clockEpochAge > 120000 ? "warn" : "muted"} />
                    <Stat label="Connection warnings" value={(multiplayer?.connectionWarnings || []).length || "none"} tone={(multiplayer?.connectionWarnings || []).length ? "bad" : "good"} hint={(multiplayer?.connectionWarnings || []).map((warning) => `${warning.name || warning.peerId}: ${warning.kind}`).join(", ") || undefined} />
                  </div>
                </section>

                <section className="fantasy-sheet-section diagnostics-section">
                  <h3 className="diagnostics-section-title">Peers</h3>
                  {snapshot.peers.length === 0 ? <div className="diagnostics-empty">No peer connections observed.</div> : (
                    <table className="diagnostics-table">
                      <thead><tr><th>Peer</th><th>State</th><th>Round trip</th><th>Avg</th><th>Last received</th><th>Last sent</th><th>Traffic</th></tr></thead>
                      <tbody>
                        {snapshot.peers.map((peer) => {
                          const lobby = lobbyPlayers.find((player) => String(player?.peerId || "") === peer.peerId);
                          return (
                            <tr key={peer.peerId}>
                              <td>{peer.name || lobby?.name || peer.peerId.slice(0, 8)}</td>
                              <td data-tone={lobby && lobby.connected === false ? "bad" : peer.state === "closed" ? "bad" : "good"}>{lobby && lobby.connected === false ? "offline" : peer.state}</td>
                              <td data-tone={tone(peer.rttMs, 300, 1000)}>{ms(peer.rttMs)}</td>
                              <td>{ms(peer.rttAvgMs)}</td>
                              <td data-tone={tone(peer.sinceReceivedMs, 5000, 10000)}>{age(peer.sinceReceivedMs)} ago</td>
                              <td>{age(peer.sinceSentMs)} ago</td>
                              <td>{peer.received} in / {peer.sent} out · {Math.round((peer.bytesIn + peer.bytesOut) / 1024)} KB</td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  )}
                </section>

                <section className="fantasy-sheet-section diagnostics-section">
                  <h3 className="diagnostics-section-title">Recent actions</h3>
                  {snapshot.traces.length === 0 ? <div className="diagnostics-empty">No actions traced yet.</div> : (
                    <div className="diagnostics-traces">
                      {snapshot.traces.slice(0, 12).map((trace) => (
                        <div key={trace.id} className="diagnostics-trace" data-outcome={trace.outcome || "open"}>
                          <div className="diagnostics-trace-head">
                            <span className="diagnostics-trace-label">{trace.label}</span>
                            <span className="diagnostics-trace-mode">{trace.mode}</span>
                            <span className="diagnostics-trace-total" data-tone={tone(trace.done ? trace.totalMs : snapshot.at - trace.startedAt, 1000, 8000)}>
                              {trace.done ? `${ms(trace.totalMs)} · ${trace.outcome}` : `${seconds(snapshot.at - trace.startedAt)} · in flight`}
                            </span>
                            <span className="diagnostics-trace-when">{new Date(trace.startedAtWall).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}</span>
                          </div>
                          <StageChips trace={trace} nowMs={snapshot.at} />
                        </div>
                      ))}
                    </div>
                  )}
                </section>

                <section className="fantasy-sheet-section diagnostics-section">
                  <h3 className="diagnostics-section-title">Recent events</h3>
                  {snapshot.events.length === 0 ? <div className="diagnostics-empty">Quiet.</div> : (
                    <ul className="diagnostics-events">
                      {snapshot.events.slice(0, 30).map((event) => (
                        <li key={`${event.kind}-${event.at}`}>
                          <span className="diagnostics-event-age">{age(snapshot.at - event.at)} ago</span>
                          <span className="diagnostics-event-kind">{event.kind}</span>
                          <span className="diagnostics-event-meta">{event.meta ? Object.entries(event.meta).map(([key, value]) => `${key}=${typeof value === "object" ? JSON.stringify(value) : value}`).join("  ") : ""}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </section>

                <div className="diagnostics-toolbar">
                  <Button type="button" variant="secondary" size="sm" className="stone-pill" onClick={handleCopy}>
                    {copied ? <Check className="size-3.5" aria-hidden="true" /> : <ClipboardCopy className="size-3.5" aria-hidden="true" />}
                    {copied ? "Copied" : "Copy report"}
                  </Button>
                  <Button type="button" variant="secondary" size="sm" className="stone-pill" onClick={handleDownload}>
                    Download report
                  </Button>
                  <Button type="button" variant="secondary" size="sm" className="stone-pill" onClick={() => resetDiagnostics()}>
                    <Eraser className="size-3.5" aria-hidden="true" /> Clear
                  </Button>
                  <span className="diagnostics-toolbar-note">Also available in the console as <code>__ironsmithDiagnostics.export()</code>.</span>
                </div>
              </div>
            );
          })() : null}
        </SheetContent>
      </Sheet>
    </>
  );
}
