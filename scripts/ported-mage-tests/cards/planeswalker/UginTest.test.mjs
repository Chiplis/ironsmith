import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/UginTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Quest for the Gravelord",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Nissa, Vastwood Seer",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: {this} deals 3 damage",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Nissa, Vastwood Seer"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Yes"
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Forest"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "-2: Create Ashaya, the Awoken World, a legendary 4/4 green Elemental creature token."
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Ashaya, the Awoken World",
          "attacker": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "-X: Exile each permanent with mana value X or less that's one or more colors"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
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
          "name": "Ugin, the Spirit Dragon",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ugin, the Spirit Dragon",
          "counter": "LOYALTY",
          "count": 6
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ashaya, the Awoken World",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Nissa, Vastwood Seer",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Quest for the Gravelord",
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
          "life": 17
        }
      ]
    }
  ]
});
