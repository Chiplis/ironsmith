# Ironsmith realtime worker

This Worker provides one authoritative Durable Object per lobby. Clients join
over WebSocket, submit idempotent actions, and resume from their last sequence
after a refresh or network interruption.

## Deploy

```bash
npm install
npx wrangler deploy
```

The CI workflow should provide `CLOUDFLARE_API_TOKEN` and
`CLOUDFLARE_ACCOUNT_ID` as secrets. The frontend endpoint is:
`wss://<worker-domain>/lobby?id=<lobby-id>`.
