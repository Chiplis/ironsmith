import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/restriction/ArrestTest.java",
  "tests": [
    {
      "name": "testArrest1",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arrest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Selesnya Guildmage",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Arrest",
          "target": "Selesnya Guildmage"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Selesnya Guildmage",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Player PlayerB must have 0 actions but found 1\")) { Assert.fail(\"Should have thrown error about cannot attack, but got:\\n\" + e.getMessage()); } } assertPermanentCount(playerA, \"Arrest\", 1)"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Saproling Token",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
