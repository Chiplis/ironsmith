import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/RenownTest.java",
  "tests": [
    {
      "name": "testKnightOfThePilgrimsRoad",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Knight of the Pilgrim's Road",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Knight of the Pilgrim's Road"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Knight of the Pilgrim's Road",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Knight of the Pilgrim's Road",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": 0,
          "attacker": "Knight of the Pilgrim's Road",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Knight of the Pilgrim's Road",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 9
        }
      ]
    },
    {
      "name": "testRelicSeeker",
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
          "name": "Relic Seeker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Veteran's Sidearm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Relic Seeker"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Relic Seeker",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Relic Seeker",
          "defender": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Relic Seeker",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Veteran's Sidearm",
          "count": 1
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
    },
    {
      "name": "testHonoredHierarch",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Honored Hierarch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Veteran's Sidearm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Honored Hierarch"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Honored Hierarch",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Honored Hierarch",
          "defender": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Honored Hierarch",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Honored Hierarch\", false)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Honored Hierarch",
          "ability": "Vigilance",
          "expected": true
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
        }
      ]
    },
    {
      "name": "testRhoxMaulers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rhox Maulers",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rhox Maulers"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Rhox Maulers",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Rhox Maulers",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": 0,
          "attacker": "Rhox Maulers",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rhox Maulers",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 4
        }
      ]
    },
    {
      "name": "testRenownGoneAfterZoneChange",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rhox Maulers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rhox Maulers"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cloudshift",
          "target": "Rhox Maulers"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Rhox Maulers",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Rhox Maulers",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": 0,
          "attacker": "Rhox Maulers",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 6
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Rhox Maulers",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "testRenownGainedGainAfterZoneChange",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
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
          "name": "Goblin Glory Chaser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Glory Chaser"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cloudshift",
          "target": "Goblin Glory Chaser"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Goblin Glory Chaser",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Goblin Glory Chaser",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 7,
          "player": 0,
          "attacker": "Goblin Glory Chaser",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 9,
          "player": 0,
          "attacker": "Goblin Glory Chaser",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 9,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Goblin Glory Chaser",
          "ability": "new MenaceAbility()",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Goblin Glory Chaser",
          "power": 2,
          "toughness": 2
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
      "name": "testScabClanBerserker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scab-Clan Berserker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Scab-Clan Berserker",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Scab-Clan Berserker",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testEnshroudingMist",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Scab-Clan Berserker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Enshrouding Mist",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Scab-Clan Berserker",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Enshrouding Mist",
          "target": "Scab-Clan Berserker"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Scab-Clan Berserker",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Scab-Clan Berserker\", false)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
