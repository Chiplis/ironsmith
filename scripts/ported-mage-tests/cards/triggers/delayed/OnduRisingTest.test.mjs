import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/delayed/OnduRisingTest.java",
  "tests": [
    {
      "name": "testLifelinkGained",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 5
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Ondu Rising",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Ondu Rising with awaken"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Mountain"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Mountain",
          "defender": 0
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Whenever "
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Ondu Rising",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Mountain",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 26
        }
      ]
    },
    {
      "name": "testNoFizzle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 5
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Ondu Rising",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Ondu Rising with awaken"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Mountain"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Mountain",
          "defender": 0
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Whenever "
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Doom Blade",
          "target": "Mountain"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Ondu Rising",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        }
      ]
    }
  ]
});
