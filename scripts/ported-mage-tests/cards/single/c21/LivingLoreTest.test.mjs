import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c21/LivingLoreTest.java",
  "tests": [
    {
      "name": "testZCC",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Lore@lore",
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
          "name": "Living Lore",
          "objectAlias": "@lore"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Living Lore",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cruel Ultimatum"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@lore",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "@lore"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Unsummon",
          "count": 1,
          "objectTarget": "Living Lore"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Living Lore",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore",
          "objectAlias": "@lore"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Living Lore",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lightning Bolt"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@lore",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "@lore",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Living Lore",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
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
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
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
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
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
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore",
          "power": 7,
          "toughness": 7
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Lore@lore",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Lore@lore.second",
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
          "name": "Living Lore",
          "objectAlias": "@lore"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Living Lore",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Chaplain's Blessing"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Chaplain's Blessing",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@lore",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Living Lore",
          "objectAlias": "@lore.second"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Living Lore",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cruel Ultimatum"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Cruel Ultimatum",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@lore.second",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "@lore",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Living Lore",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
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
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "COMBAT_DAMAGE",
          "source": "Chaplain's Blessing",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "COMBAT_DAMAGE"
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
          "name": "Chaplain's Blessing",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@lore.second",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
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
