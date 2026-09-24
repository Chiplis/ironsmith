import test from "node:test";
import assert from "node:assert/strict";
import { resolveAssetUrl, resolveCardAssetUrl } from "../src/lib/card-art-url.js";

test("card assets stay under the deployed base path", () => {
  assert.equal(
    resolveCardAssetUrl("counterspell", { baseUrl: "/ironsmith/" }),
    "http://localhost/ironsmith/cards/counterspell.json",
  );
  assert.equal(
    resolveAssetUrl("/cards/index.json", { baseUrl: "/ironsmith/" }),
    "http://localhost/ironsmith/cards/index.json",
  );
});
