// A branch is a lossless engine session, not a redacted wire checkpoint.
// Worker calls enter and leave it in one queue task; network awaits never hold
// the visible engine in the verification branch.
export async function inRuntimeBranch(game, handle, operation, reportPhase = () => {}) {
  if (handle == null) return operation();
  reportPhase('branch_enter');
  game.exchangeRuntimeSavepoint(handle);
  try {
    reportPhase('branch_operation');
    return await operation();
  } finally {
    reportPhase('branch_exit');
    game.exchangeRuntimeSavepoint(handle);
  }
}

export function attachRuntimeBranches(proxy, { call, createProxy, ready }) {
  proxy.forkRuntimeBranch = async () => {
    if (!ready()) throw new Error('Lossless runtime branches are unavailable');
    const generation = proxy.runtimeGeneration;
    const handle = await call('createRuntimeSavepoint', []);
    if (generation !== proxy.runtimeGeneration) throw new Error('Engine instance has expired');
    let released = false;
    const branchCall = (method, args) => {
      if (released) return Promise.reject(new Error('Runtime branch has been released'));
      if (generation !== proxy.runtimeGeneration) return Promise.reject(new Error('Engine instance has expired'));
      return call(method, args, handle);
    };
    const branch = createProxy(branchCall);
    branch.supportsRuntimeSavepoints = true;
    branch.runtimeBranch = handle;
    // Background snapshots must never masquerade as a current visible snapshot.
    branch.isCurrentSnapshot = () => false;
    branch.adoptSnapshotVersion = () => {};
    branch.copyToVisible = () => {
      if (released) return Promise.reject(new Error('Runtime branch has been released'));
      if (generation !== proxy.runtimeGeneration) return Promise.reject(new Error('Engine instance has expired'));
      return call('copyRuntimeSavepoint', [handle]);
    };
    branch.release = async () => {
      if (released) return;
      released = true;
      if (generation !== proxy.runtimeGeneration) return;
      await call('releaseRuntimeSavepoint', [handle]);
    };
    return branch;
  };
}
