import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/usg/NoRestForTheWickedTest.java",
  "tests": [
    {
      "name": "testSacrifice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Royal Assassin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sengir Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Flowering Lumberknot",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Moorland Inquisitor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "No Rest for the Wicked"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Royal Assassin",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Sengin Vampire",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Moorland Inquisitor",
          "attacker": "Memnite"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice {this}: Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn."
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "testSacrificeAfterDying",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Royal Assassin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sengir Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Flowering Lumberknot",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Moorland Inquisitor",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Royal Assassin",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Sengin Vampire",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Moorland Inquisitor",
          "attacker": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "No Rest for the Wicked"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice {this}: Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn."
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
          "name": "No Rest for the Wicked",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "testTakeControlThenSacrifice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Beacon of Unrest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Moorland Inquisitor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Moorland Inquisitor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "No Rest for the Wicked"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Moorland Inquisitor",
          "attacker": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Beacon of Unrest",
          "target": "Memnite"
        },
        {
          "op": "attack",
          "turn": 7,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 7,
          "player": 1,
          "blocker": "Moorland Inquisitor",
          "attacker": "Memnite"
        },
        {
          "op": "activateAbility",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice {this}: Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn."
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "No Rest for the Wicked",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Memnite",
          "count": 0
        }
      ]
    }
  ]
});
