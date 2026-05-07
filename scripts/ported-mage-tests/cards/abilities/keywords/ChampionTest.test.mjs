import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ChampionTest.java",
  "tests": [
    {
      "name": "testChampionCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Crafter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Crafter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Roughrider"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lightning Crafter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        }
      ]
    },
    {
      "name": "testExiledCreatureReturns",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Crafter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Crafter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Roughrider"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: {this} deals 3 damage to ",
          "target": "Lightning Crafter"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lightning Crafter",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Goblin Roughrider",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Crafter",
          "count": 1
        }
      ]
    },
    {
      "name": "testMistbindClique",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zephyr Sprite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mistbind Clique",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mistbind Clique"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Zephyr Sprite"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mistbind Clique",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Zephyr Sprite",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Plains\", true)"
        }
      ]
    }
  ]
});
