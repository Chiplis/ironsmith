# Publishing `ironsmith-wasm`

The package version comes from `crates/ironsmith-wasm/Cargo.toml`; the generated package manifest is never edited by hand.

Before the first public release, add the repository's chosen license. This repository currently has no license file, so the package template deliberately does not claim one.

## Local release check

```sh
node scripts/build-npm-package.mjs
node scripts/verify-npm-package.mjs
```

The build uses `wasm-lean` with default Cargo features disabled, runs the release optimizer, and writes only generated output to `target/npm/ironsmith-wasm`. Verification instantiates the artifact in Node, exercises external and Manabrew card loading, inspects `npm pack --dry-run`, and bundles a temporary Vite consumer.

Use `--no-opt` only for a faster local iteration build:

```sh
node scripts/build-npm-package.mjs --no-opt
```

## GitHub release

1. Update the crate version and `Cargo.lock`.
2. Push a tag matching the version, for example `ironsmith-wasm-v0.1.0`.
3. The `Publish ironsmith-wasm` workflow rebuilds and verifies the artifact, checks the tag against the crate version, and publishes it with npm provenance.

## One-time npm bootstrap

npm requires a package to exist before it can have a trusted publisher. For the
first release only, create a short-lived granular npm token that can create the
unscoped `ironsmith-wasm` package, enable bypass 2FA, and save it as the
`NPM_TOKEN` Actions secret in `Chiplis/ironsmith`. Push the first release tag and
wait for the workflow to publish the package.

Immediately afterward, open the `ironsmith-wasm` package settings on npm and add
this trusted publisher:

- Provider: GitHub Actions
- Organization or user: `Chiplis`
- Repository: `ironsmith`
- Workflow filename: `publish-npm.yml`
- Environment: blank
- Allowed action: `npm publish`

The same configuration can be created from an authenticated current npm CLI:

```sh
npm trust github ironsmith-wasm \
  --repository Chiplis/ironsmith \
  --file publish-npm.yml \
  --allow-publish
```

After the trusted publisher is saved, delete the `NPM_TOKEN` GitHub secret and
revoke the npm bootstrap token. Future release tags authenticate through GitHub
OIDC. For maximum protection, set the package's publishing access to require 2FA
and disallow traditional tokens after verifying the first OIDC release.
