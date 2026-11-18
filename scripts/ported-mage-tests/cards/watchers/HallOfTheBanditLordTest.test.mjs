import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/HallOfTheBanditLordTest.java",
  "tests": [
    {
      "name": "testGainsHaste",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hall of the Bandit Lord",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "source": "Hall of the Bandit Lord",
          "ability": "Add {C}",
          "player": 0,
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 1
        },
        {
          "op": "castSpell",
          "name": "Goblin Roughrider",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Goblin Roughrider",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttackDefenders",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "Goblin Roughrider",
          "defenders": [
            1
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Goblin Roughrider"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            }
          ]
        }
      ]
    },
    {
      "name": "testNotGainsHaste",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hall of the Bandit Lord",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ember Hauler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ember Hauler"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Ember Hauler",
          "ability": "Haste",
          "expected": false
        }
      ]
    },
    {
      "name": "testGainsHasteCopied",
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
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hall of the Bandit Lord@hall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mirror Gallery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vesuva@vesuva",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            {
              "description": "Enter as a copy of Hall of the Bandit Lord",
              "object": "@hall"
            }
          ]
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vesuva"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hall of the Bandit Lord",
          "count": 2
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "@vesuva",
          "tapped": true
        },
        {
          "op": "activateManaAbility",
          "source": "@hall",
          "ability": "Add {C}",
          "player": 0,
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "castSpell",
          "name": "Goblin Piker",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Goblin Piker",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttackDefenders",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "Goblin Piker",
          "defenders": [
            1
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Goblin Piker"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            }
          ]
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "activateManaAbility",
          "source": "@vesuva",
          "ability": "Add {C}",
          "player": 0,
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "castSpell",
          "name": "Feral Maaka",
          "player": 0,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Feral Maaka",
          "count": 1,
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Feral Maaka"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "expected": true,
              "value": "haste"
            }
          ]
        }
      ]
    }
  ]
});
