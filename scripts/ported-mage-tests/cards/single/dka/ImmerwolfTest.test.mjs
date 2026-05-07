import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/ImmerwolfTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Afflicted Deserter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Immerwolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Afflicted Deserter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Werewolf Ransacker",
          "count": 1
        }
      ]
    },
    {
      "name": "testCard1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Afflicted Deserter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Immerwolf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Afflicted Deserter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Werewolf Ransacker",
          "count": 1
        }
      ]
    }
  ]
});
