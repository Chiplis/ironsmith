import assert from "node:assert/strict";
import test from "node:test";
import { hideEmptyDefinitionFields } from "../src/lib/compiled-definition-display.js";

test("hides default fields while preserving populated fields and enum payloads", () => {
  const raw = `CardDefinition {
    optional: None,
    enabled: false,
    effects: [],
    metadata: {},
    text: "",
    count: 0,
    ratio: 0.0,
    name: "None",
    active: true,
    loyalty: Some(0),
    effects: [
        false,
        None,
        "text: false,",
    ],
}`;
  assert.equal(hideEmptyDefinitionFields(raw), `CardDefinition {
    name: "None",
    active: true,
    loyalty: Some(0),
    effects: [
        false,
        None,
        "text: false,",
    ],
}`);
  assert.equal(hideEmptyDefinitionFields(), "");
});
