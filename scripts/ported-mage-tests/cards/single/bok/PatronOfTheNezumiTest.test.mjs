import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bok/PatronOfTheNezumiTest.java",
  "tests": [
    {
      "name": "testEffectWithTokens",
      "operations": [
        {
          "op": "unsupported",
          "source": "this.setupTest()"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elspeth, Sun's Champion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "+1: Create three"
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "+1: Create three"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elesh Norn, Grand Cenobite"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        }
      ]
    },
    {
      "name": "testEffectWithRIP",
      "operations": [
        {
          "op": "unsupported",
          "source": "this.setupTest()"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rest in Peace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elesh Norn, Grand Cenobite"
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
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 5
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
    },
    {
      "name": "testEffectWithCreatures",
      "operations": [
        {
          "op": "unsupported",
          "source": "this.setupTest()"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elesh Norn, Grand Cenobite"
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
          "count": 1,
          "name": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    }
  ]
});
