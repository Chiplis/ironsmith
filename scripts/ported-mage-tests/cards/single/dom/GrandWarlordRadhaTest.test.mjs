import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dom/GrandWarlordRadhaTest.java",
  "tests": [
    {
      "name": "testMana",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lightning Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grand Warlord Radha",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Living Twister",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Raging Goblin",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Lightning Elemental",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grand Warlord Radha",
          "defender": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "colors",
          "choices": [
            "Red",
            "Red",
            "Green"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Grand Warlord Radha",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "colors": "R",
          "amount": 2,
          "holdStack": true
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "colors": "G",
          "amount": 1,
          "holdStack": true
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 2
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "G",
          "amount": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Living Twister"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Living Twister",
          "power": 2,
          "toughness": 5
        }
      ]
    }
  ]
});
