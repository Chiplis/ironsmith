import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/ManaSourceTest.java",
  "tests": [
    {
      "name": "testCantCastWithCreatureCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Simian Spirit Guide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Myr Superion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Manakin",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Exile",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Myr Superion"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Myr Superion\")) { Assert.fail(\"must not have throw error about bad targets, but got:\\n\" + e.getMessage()); } } assertExileCount(\"Simian Spirit Guide\", 1)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Myr Superion",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Myr Superion",
          "count": 1
        }
      ]
    }
  ]
});
