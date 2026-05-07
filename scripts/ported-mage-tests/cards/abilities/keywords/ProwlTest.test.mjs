import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ProwlTest.java",
  "tests": [
    {
      "name": "test_ProwlNormal",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Auntie's Snitch",
          "count": 1
        },
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
          "player": 0,
          "name": "Bloodmark Mentor",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Auntie's Snitch",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Bloodmark Mentor",
          "defender": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Auntie's Snitch",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Auntie's Snitch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Prowl alternative cost: {1}{B} (source: Auntie's Snitch"
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
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bloodmark Mentor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Auntie's Snitch",
          "count": 1
        }
      ]
    },
    {
      "name": "test_ProwlWithCostReduce",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Auntie's Snitch",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Warchief",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Auntie's Snitch",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Goblin Warchief",
          "defender": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Auntie's Snitch",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Auntie's Snitch"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Prowl alternative cost: {1}{B} (source: Auntie's Snitch"
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
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Warchief",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Auntie's Snitch",
          "count": 1
        }
      ]
    },
    {
      "name": "test_ProwlWithGainAbilityControlledSpellsEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hunting Velociraptor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thrasta, Tempest's Roar",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Thrasta",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Hunting Velociraptor",
          "defender": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Thrasta",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Thrasta, Tempest's Roar"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Prowl alternative cost: {2}{R} (source: Thrasta, Tempest's Roar"
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
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 3"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hunting Velociraptor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Thrasta, Tempest's Roar",
          "count": 1
        }
      ]
    }
  ]
});
