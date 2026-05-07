import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/rex/IndoraptorThePerfectHybridTests.java",
  "tests": [
    {
      "name": "testNoDamageETB",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "power": 3,
          "toughness": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "testDamageETB",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "power": 6,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "counter": "P1P1",
          "count": 3
        }
      ]
    },
    {
      "name": "testEnrageNoCreaturesToSac",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "power": 6,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 3 - (3 + 3)"
        }
      ]
    },
    {
      "name": "testEnrageWithCreaturesToSac",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Memnite"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "power": 6,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 3"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "testEnrageWithCreaturesToSacChooseNo",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Indoraptor, the Perfect Hybrid"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "No"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "power": 6,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Indoraptor, the Perfect Hybrid",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 3 - (3 + 3)"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        }
      ]
    }
  ]
});
