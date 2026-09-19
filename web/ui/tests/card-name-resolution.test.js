import assert from "node:assert/strict";
import test from "node:test";
import { resolveCardNameForGame } from "../src/lib/card-name-resolution.js";

test("localized name resolves to the embedded canonical card before mutation", async () => {
  const calls = [];
  const game = {
    async filterKnownCardNames(names) {
      calls.push(names);
      return names.filter((name) => name === "Raise Dead");
    },
  };
  const result = await resolveCardNameForGame({
    game,
    cardName: "Alzar a los muertos",
    locale: "es",
    resolveExternal: async () => ({ canonicalName: "Raise Dead", oracleId: "oracle-raise" }),
  });

  assert.equal(result.status, "available");
  assert.equal(result.canonicalName, "Raise Dead");
  assert.deepEqual(calls, [["Alzar a los muertos"], ["Raise Dead"]]);
});

test("known English name stays in the local registry and avoids scraping", async () => {
  let externalCalls = 0;
  const result = await resolveCardNameForGame({
    game: { filterKnownCardNames: async (names) => names },
    cardName: "Raise Dead",
    locale: "es",
    resolveExternal: async () => { externalCalls += 1; return null; },
  });

  assert.equal(result.status, "available");
  assert.equal(result.source, "registry");
  assert.equal(externalCalls, 0);
});

test("a resolved card outside the embedded registry is never treated as addable", async () => {
  const result = await resolveCardNameForGame({
    game: { filterKnownCardNames: async () => [] },
    cardName: "Vanille, Cheerful L'cie",
    locale: "en",
    resolveExternal: async () => ({
      canonicalName: "Vanille, Cheerful L'cie",
      oracleId: "oracle-vanille",
    }),
  });

  assert.equal(result.status, "not-embedded");
  assert.equal(result.canonicalName, "Vanille, Cheerful L'cie");
});

test("a known face never substitutes for an unavailable whole card", async () => {
  const result = await resolveCardNameForGame({
    game: { filterKnownCardNames: async (names) => names.filter((name) => name === "Raise Dead") },
    cardName: "Cheerful Osteomancer",
    locale: "en",
    resolveExternal: async () => ({
      canonicalName: "Cheerful Osteomancer // Raise Dead",
      aliases: ["Cheerful Osteomancer", "Raise Dead"],
    }),
  });

  assert.equal(result.status, "not-embedded");
  assert.equal(result.canonicalName, "Cheerful Osteomancer // Raise Dead");
});
