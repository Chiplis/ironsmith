import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lcc/RipplesOfPotentialTest.java",
  "tests": [
    {
      "name": "test_RipplesOfPotential",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blast Zone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arcbound Javelineer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chandra, Pyromaster",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Atraxa's Skitterfang",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ripples of Potential",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ripples of Potential",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Ripples of Potential",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "proliferate",
          "description": "Choose permanents and/or players to proliferate",
          "choices": [
            "Permanent: Blast Zone",
            "Permanent: Arcbound Javelineer",
            "Permanent: Chandra, Pyromaster",
            "Permanent: Atraxa's Skitterfang"
          ]
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blast Zone^Arcbound Javelineer^Chandra, Pyromaster"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Blast Zone",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Arcbound Javelineer",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chandra, Pyromaster",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Atraxa's Skitterfang",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Atraxa's Skitterfang",
          "counter": "OIL",
          "count": 4
        }
      ]
    }
  ]
});
