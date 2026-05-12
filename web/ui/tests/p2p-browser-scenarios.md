# P2P Browser Audit Scenarios

Run with:

```bash
cd web/ui
pnpm test:p2p-browser
```

The runner launches four isolated Chromium browser contexts, injects the production browser audit module, gives each player its own WebCrypto signing key and committed library manifest, and routes `apply_action` messages through a simulated P2P mesh. It intentionally does not use a centralized sequencer.

## Covered Situations

| Situation | Expected result |
| --- | --- |
| Four honest players | Every browser accepts the same actor-signed action log, verifies the transcript, and converges on one audit state hash. |
| Non-active player disconnects | Connected peers continue only while the disconnected player is not required for priority; the UI warning path is exercised; the returning peer replays the canonical log and converges. |
| Active priority holder disconnects | The game stalls at that player. A different player cannot submit a replacement action because the signed actor does not match the priority holder. |
| Active priority holder times out | After the agreed match clock expires, another peer can submit a signed timeout-forfeit action for the stalled priority holder. |
| Early timeout claim | Rejected until the receiving browser's local match clock has actually expired. |
| Multiple players disconnect | Remaining peers keep the canonical log for actions they can legally process; returning peers replay missed actions and converge. |
| Former host censors or withholds delivery | The actor-signed action reaches peers through mesh relay without host approval or host sequencing. |
| Peer forges an action for another player | Rejected because `signer !== actor`; no state advances. |
| Peer tampers with a signed command | Rejected because the command no longer matches the signed audit envelope. |
| Peer reveals a card not matching its committed library slot | Rejected by deck-opening verification against the public commitment manifest. |
| Peer tampers with an RNG reveal | Rejected because the revealed nonce no longer matches the signed commitment transcript. |
| Peer acts out of turn | Rejected even if the signature is valid, because the current priority holder is different. |
| Peer skips an audit sequence | Rejected as a sequence gap; the correct behavior is to request/resync missing history. |
| Peer replays a duplicate action | Treated as idempotent duplicate traffic; state is not mutated twice. |

## Not Modeled Here

This browser runner validates the live audit, action relay, commitment opening, RNG transcript rejection, and disconnect/resync behavior in real browser crypto contexts. It does not try to prove the lower-level mental-poker shuffle itself; that is covered by the WASM ziffle tests. The separate PeerJS E2E harness covers real PeerJS signaling/reconnect behavior through the React lobby.
