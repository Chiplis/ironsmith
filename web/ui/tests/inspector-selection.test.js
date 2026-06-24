import test from "node:test";
import assert from "node:assert/strict";
import { resolveInspectorObjectId } from "../src/lib/inspector-selection.js";

test("selected inspector object wins over hover", () => {
  assert.equal(
    resolveInspectorObjectId({
      selectedObjectId: 101,
      hoveredObjectId: 202,
    }),
    "101"
  );
});

test("pinned inspector object wins over hover when no selected object is active", () => {
  assert.equal(
    resolveInspectorObjectId({
      pinnedObjectId: 303,
      hoveredObjectId: 404,
    }),
    "303"
  );
});

test("hover drives inspector only when no selection lock is active", () => {
  assert.equal(
    resolveInspectorObjectId({
      hoveredObjectId: 505,
    }),
    "505"
  );
});
