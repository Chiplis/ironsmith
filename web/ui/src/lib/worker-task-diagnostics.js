// Publish before synchronous work starts: a frozen worker cannot answer a
// diagnostics request. Only method names, IDs and timings cross this channel.
export function createWorkerTaskDiagnostics({ publish, now = () => performance.now(),
  wallNow = () => Date.now(), maxQueued = 12, maxPhases = 24 } = {}) {
  let sequence = 0, generation = 0, active = null;
  const tasks = new Map(), queue = [];
  const describe = task => task && ({ taskId: task.taskId, requestId: task.requestId,
    kind: task.kind, method: task.method, commandType: task.commandType,
    runtimeBranch: task.runtimeBranch, phase: task.phase,
    startedAtWall: task.startedAtWall, phaseStartedAtWall: task.phaseStartedAtWall,
    elapsedMs: Math.max(0, now() - task.startedAt),
    phaseElapsedMs: Math.max(0, now() - task.phaseStartedAt),
    preparationMs: task.preparationMs, preparationReady: task.preparationReady,
    nodeBudget: task.nodeBudget, spentNodes: task.spentNodes });
  const snapshot = () => {
    const queued = new Set(queue), outsideQueue = [];
    for (const task of tasks.values()) {
      if (task !== active && !queued.has(task)) outsideQueue.push(describe(task));
      if (outsideQueue.length >= maxQueued) break;
    }
    return { generation, sentAtWall: wallNow(), active: describe(active),
      queueDepth: queue.length, pendingCount: tasks.size,
      queued: queue.slice(0, maxQueued).map(describe), outsideQueue };
  };
  const emit = completed => {
    try { publish?.({ type: 'workerDiagnostics', state: snapshot(), ...(completed ? { completed } : {}) }); }
    catch { /* Diagnostics must never prevent a command or its response. */ }
  };
  const phase = (task, name, details = {}) => {
    if (!tasks.has(task.taskId)) return;
    const at = now();
    if (task.phases.length < maxPhases) task.phases.push({ phase: task.phase, ms: Math.max(0, at - task.phaseStartedAt) });
    else task.omittedPhases++;
    task.phase = name; task.phaseStartedAt = at; task.phaseStartedAtWall = wallNow();
    if (Number.isFinite(details.nodeBudget)) task.nodeBudget = details.nodeBudget;
    if (Number.isFinite(details.spentNodes)) task.spentNodes = details.spentNodes;
    emit();
  };
  return {
    create(metadata = {}) {
      const at = now();
      const task = { taskId: ++sequence, requestId: metadata.requestId ?? null,
        kind: metadata.kind || 'command', method: metadata.method ?? null,
        commandType: metadata.commandType ?? null, runtimeBranch: metadata.runtimeBranch ?? null,
        startedAt: at, startedAtWall: wallNow(), phase: 'received', phaseStartedAt: at,
        phaseStartedAtWall: wallNow(), phases: [], omittedPhases: 0 };
      tasks.set(task.taskId, task); return task;
    },
    enqueue(task) { queue.push(task); phase(task, 'queued'); },
    start(task) {
      const index = queue.indexOf(task); if (index >= 0) queue.splice(index, 1);
      active = task; phase(task, 'starting');
    },
    leaveQueue(task) { if (active === task) active = null; },
    phase,
    phaseActive(name, details) { if (active) phase(active, name, details); },
    prepared(task, startedAt) {
      task.preparationReady = true; task.preparationMs = Math.max(0, now() - startedAt); emit();
    },
    finish(task, outcome = 'ok') {
      if (!tasks.has(task.taskId)) return;
      const completed = { ...describe(task), outcome, phases: [...task.phases,
        { phase: task.phase, ms: Math.max(0, now() - task.phaseStartedAt) }], omittedPhases: task.omittedPhases };
      tasks.delete(task.taskId);
      const index = queue.indexOf(task); if (index >= 0) queue.splice(index, 1);
      if (active === task) active = null;
      emit(completed);
    },
    runSync(metadata, operation) {
      const parent = active, task = this.create(metadata);
      this.start(task);
      let outcome = 'error';
      try { const result = operation(); outcome = 'ok'; return result; }
      finally { active = parent; this.finish(task, outcome); }
    },
    reset() { generation++; active = null; tasks.clear(); queue.length = 0; emit(); },
    snapshot,
  };
}
