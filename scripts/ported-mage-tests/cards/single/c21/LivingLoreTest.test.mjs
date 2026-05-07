import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c21/LivingLoreTest.java",
  "tests": [
    {
      "name": "testZCC",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 9
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Cruel Ultimatum"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Living Lore"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "Living Lore"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Lightning Bolt"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Living Lore"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Living Lore",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testPullFromEternity",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pull from Eternity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Cruel Ultimatum"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Living Lore"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pull from Eternity",
          "target": "Cruel Ultimatum"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pull from Eternity",
          "count": 1
        }
      ]
    },
    {
      "name": "testLivingLoreNotSharingExile",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Lore",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Chaplain's Blessing",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Chaplain's Blessing"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Cruel Ultimatum"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Living Lore",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Living Lore",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Chaplain's Blessing",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        }
      ]
    }
  ]
});
