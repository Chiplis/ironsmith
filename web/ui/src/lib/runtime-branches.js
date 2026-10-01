// A branch is a lossless engine session, not a redacted wire checkpoint.
// Worker calls enter and leave it in one queue task; network awaits never hold
// the visible engine in the verification branch.
export async function inRuntimeBranch(game, handle, operation) {
  if (handle == null) return operation();
  game.exchangeRuntimeSavepoint(handle);
  try {
    return await operation();
  } finally {
    game.exchangeRuntimeSavepoint(handle);
  }
}

export function attachRuntimeBranches(proxy, { call, createProxy, ready }) {
  proxy.forkRuntimeBranch = async () => {
    if (!ready()) throw new Error('Lossless runtime branches are unavailable');
    const handle = await call('createRuntimeSavepoint', []);
    let released = false;
    const branchCall = (method, args) => {
      if (released) return Promise.reject(new Error('Runtime branch has been released'));
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
      return call('copyRuntimeSavepoint', [handle]);
    };
    branch.release = async () => {
      if (released) return;
      released = true;
      await call('releaseRuntimeSavepoint', [handle]);
    };
    return branch;
  };
}
