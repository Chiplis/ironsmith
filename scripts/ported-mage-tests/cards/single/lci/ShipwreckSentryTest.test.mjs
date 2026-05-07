import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/ShipwreckSentryTest.java",
  "tests": [
    {
      "name": "testCantAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shipwreck Sentry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Shipwreck Sentry",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); } catch (AssertionError e) { Assert.assertTrue(\"No artifact entered\", e.getMessage().contains(\"Can't find available command - attack:Shipwreck Sentry$defendingPlayer=PlayerB\")); }"
        }
      ]
    },
    {
      "name": "testCanAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shipwreck Sentry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Darksteel Relic"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Shipwreck Sentry",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Shipwreck Sentry",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    }
  ]
});
