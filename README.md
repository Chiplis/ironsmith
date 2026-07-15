Magic: The Gathering engine supporting automatic oracle text parsing and custom card compilation via natural language and 4-way multiplayer backed by Zero Knowledge proofs.

~26k cards supported, with more to come soon!

https://chiplis.com/ironsmith

## Browser / npm package

Build and verify the lean `ironsmith-wasm` npm artifact with:

```sh
node scripts/build-npm-package.mjs
node scripts/verify-npm-package.mjs
```

Consumer usage and card-loading details are documented in [the package README](npm/ironsmith-wasm/README.md). Release setup is documented in [the publishing guide](npm/ironsmith-wasm/PUBLISHING.md).
