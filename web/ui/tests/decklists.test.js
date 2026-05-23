import assert from "node:assert/strict";
import test from "node:test";

import {
  parseDeckList,
  parseSideboardList,
} from "../src/lib/decklists.js";

test("parseDeckList strips common print metadata from card names", () => {
  assert.deepEqual(
    parseDeckList([
      "1 Beast Within (clu) 165",
      "1 Beast Within [NPH] 103",
      "1 Beast Within [nph:103]",
      "1 Beast Within (CMM) 294 *F*",
    ].join("\n")),
    ["Beast Within", "Beast Within", "Beast Within", "Beast Within"],
  );
});

test("parseSideboardList strips common print metadata from card names", () => {
  assert.deepEqual(
    parseSideboardList([
      "1 Forest",
      "Sideboard",
      "1 Beast Within (clu) 165",
      "1 Beast Within [NPH] 103",
    ].join("\n")),
    ["Beast Within", "Beast Within"],
  );
});
