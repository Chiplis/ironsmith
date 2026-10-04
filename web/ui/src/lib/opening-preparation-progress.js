// Progress is sent before a command's signed transcript entry commits. An
// opening prepared locally is not yet an authorized peer-visible reveal:
// applying the command or obtaining its proof/quorum can still fail. Keep
// card identities in the actor's local preview and the normal action payload.
export function openingPreparationProgress({ progressCurrent, progressTotal } = {}) {
  return {
    operation: "Preparing public openings",
    ...(Number.isSafeInteger(progressCurrent) && progressCurrent >= 0
      ? { progressCurrent }
      : {}),
    ...(Number.isSafeInteger(progressTotal) && progressTotal >= 0
      ? { progressTotal }
      : {}),
  };
}
