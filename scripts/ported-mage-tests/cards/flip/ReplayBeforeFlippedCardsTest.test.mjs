import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/flip/ReplayBeforeFlippedCardsTest.java",
  "tests": [
    {
      "name": "testHanweirMilitiaCaptain",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hanweir Militia Captain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hanweir Militia Captain"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hanweir Militia Captain",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Westvale Cult Leader",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Westvale Cult Leader",
          "power": 5,
          "toughness": 5
        }
      ]
    },
    {
      "name": "testHanweirMilitiaCaptainReturned",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hanweir Militia Captain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Just the Wind",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hanweir Militia Captain"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DRAW",
          "player": 1,
          "name": "Just the Wind",
          "target": "Westvale Cult Leader"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hanweir Militia Captain"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Just the Wind",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hanweir Militia Captain",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hanweir Militia Captain",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
