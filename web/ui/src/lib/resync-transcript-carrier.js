// Recovery carries authenticated actions, never another seat's engine state.
export function assertResyncTranscriptCarrier(message, { trusted, verified }) {
  if (!trusted && !verified) throw new Error('Resync replay is not supported for this security mode');
  if (message?.replayOnly !== true) throw new Error('Resync requires a replay-only transcript');
  if (message.checkpoint != null) throw new Error('Resync cannot contain serialized engine state');
  if (message.resyncEnvelope?.checkpointSequence != null) throw new Error('Resync cannot claim a checkpoint sequence');
}
