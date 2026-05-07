import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/fin/SorceresssSchemesTest.java",
  "tests": [
    {
      "name": "test_target_in_graveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sorceress's Schemes",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 0,
          "name": "Deep Analysis",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sorceress's Schemes",
          "target": "Lightning Bolt"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_target_in_exile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sorceress's Schemes",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 0,
          "name": "Deep Analysis",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sorceress's Schemes",
          "target": "Deep Analysis"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Deep Analysis",
          "count": 1
        }
      ]
    },
    {
      "name": "test_auto_target_in_graveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sorceress's Schemes",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sorceress's Schemes"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_auto_target_in_exile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sorceress's Schemes",
          "count": 1
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
          "zone": "EXILED",
          "player": 0,
          "name": "Deep Analysis",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sorceress's Schemes"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Deep Analysis",
          "count": 1
        }
      ]
    },
    {
      "name": "test_various_nontarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sorceress's Schemes",
          "count": 1
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Bloomvine Regent",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "EXILED",
          "player": 1,
          "name": "Deep Analysis",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Sorceress's Schemes",
          "expected": false
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
        }
      ]
    }
  ]
});
