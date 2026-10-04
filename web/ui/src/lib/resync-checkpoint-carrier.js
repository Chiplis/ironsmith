// Replay-only Verified messages still require the signed genesis, action log
// and resync envelope checks in the receiver. This validates only the carrier.
export function assertResyncCheckpointCarrier(message, { trusted, verified }) {
  if (message?.replayOnly === true) {
    if (!trusted && !verified) {
      throw new Error("Resync replay is not supported for this security mode");
    }
    if (message.checkpoint != null) {
      throw new Error("Replay-only resync cannot contain a WASM checkpoint");
    }
    if (verified && message.resyncEnvelope?.checkpointSequence != null) {
      throw new Error("Replay-only resync cannot claim a checkpoint sequence");
    }
    return;
  }
  if (!message?.checkpoint || typeof message.checkpoint !== "object"
      || Array.isArray(message.checkpoint)) {
    throw new Error("Resync payload is missing WASM checkpoint");
  }
}
