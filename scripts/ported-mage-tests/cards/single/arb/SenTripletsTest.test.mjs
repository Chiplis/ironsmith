import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/arb/SenTripletsTest.java",
  "tests": [
    {
      "name": "testCastSpell",
      "operations": [
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Darksteel Relic"
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sen Triplets",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Darksteel Relic",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testCantActivate",
      "operations": [
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Can't find ability to activate command: {T}\")) { Assert.fail(\"must throw error about bad targets, but got:\\n\" + e.getMessage()); } }"
        }
      ]
    },
    {
      "name": "testCantCast",
      "operations": [
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Lightning Bolt$targetPlayer=PlayerA\")) { Assert.fail(\"must throw error about bad targets, but got:\\n\" + e.getMessage()); } }"
        }
      ]
    }
  ]
});
