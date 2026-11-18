import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/eld/SyrGwynHeroOfAshvaleTest.java",
  "tests": [
    {
      "name": "equipKnightTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Behemoth Sledge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "count": 1
        },
        {
          "op": "activateAbility",
          "player": 0,
          "source": "Behemoth Sledge",
          "ability": "{0}: Attach objects.",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Syr Gwyn, Hero of Ashvale"
        },
        {
          "op": "assertStackObject",
          "source": "Behemoth Sledge",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Syr Gwyn, Hero of Ashvale"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Behemoth Sledge",
          "target": "Syr Gwyn, Hero of Ashvale",
          "targetPlayer": 0,
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "power": 7,
          "toughness": 7
        }
      ]
    },
    {
      "name": "equipKnightTestInstantSpeed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Behemoth Sledge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Leonin Shikari",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "count": 1
        },
        {
          "op": "activateAbility",
          "player": 0,
          "source": "Behemoth Sledge",
          "ability": "{0}: Attach objects.",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "target": "Syr Gwyn, Hero of Ashvale"
        },
        {
          "op": "assertStackObject",
          "source": "Behemoth Sledge",
          "count": 1,
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "abilityKind": "Activated",
          "objectTarget": "Syr Gwyn, Hero of Ashvale"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Behemoth Sledge",
          "target": "Syr Gwyn, Hero of Ashvale",
          "targetPlayer": 0,
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Syr Gwyn, Hero of Ashvale",
          "power": 7,
          "toughness": 7
        }
      ]
    }
  ]
});
