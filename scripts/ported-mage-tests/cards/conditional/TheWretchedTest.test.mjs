import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/TheWretchedTest.java",
  "tests": [
    {
      "name": "testGainControl_One_NoRegenThusNothingIsRemovedFromCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Living Wall",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "The Wretched",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Pine Needles",
          "attacker": "The Wretched"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Living Wall",
          "attacker": "The Wretched"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Living Wall",
          "count": 1
        }
      ]
    },
    {
      "name": "testGainControl_One_RegenWhichRemovesBlockerFromCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bad Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Living Wall",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "The Wretched",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Pine Needles",
          "attacker": "The Wretched"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Living Wall",
          "attacker": "The Wretched"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "ability": "{G}: Regenerate {this}."
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Living Wall",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        }
      ]
    },
    {
      "name": "testLoseControlOfTheWretched",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Living Wall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Control Magic",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "The Wretched",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Pine Needles",
          "attacker": "The Wretched"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Living Wall",
          "attacker": "The Wretched"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Control Magic",
          "target": "The Wretched"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wall of Pine Needles",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Living Wall",
          "count": 1
        }
      ]
    },
    {
      "name": "testRegenTheWretchedThusRemovingFromCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Regenerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wall of Spears",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "The Wretched",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Pine Needles",
          "attacker": "The Wretched"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wall of Spears",
          "attacker": "The Wretched"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Regenerate",
          "target": "The Wretched"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Wretched",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wall of Pine Needles",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wall of Spears",
          "count": 1
        }
      ]
    }
  ]
});
