import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/WonderTest.java",
  "tests": [
    {
      "name": "testCardWithAllConditionsMet",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Wonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corpse Traders",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Llanowar Elves",
          "count": 1
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
          "name": "Elite Vanguard",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Corpse Traders",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Llanowar Elves",
          "ability": "Flying",
          "expected": false
        },
        {
          "op": "unsupported",
          "source": "for (Card card : playerA.getGraveyard().getCards(currentGame)) { if (card.getName().equals(\"Runeclaw Bear\")) { Assert.assertFalse(card.hasAbility(FlyingAbility.getInstance(), currentGame)); } }"
        }
      ]
    },
    {
      "name": "testNoIsland",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Wonder",
          "count": 1
        },
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
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
          "name": "Elite Vanguard",
          "count": 1
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
          "name": "Elite Vanguard",
          "ability": "Flying",
          "expected": false
        }
      ]
    },
    {
      "name": "testOtherZones",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Wonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Wonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
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
          "name": "Elite Vanguard",
          "ability": "Flying",
          "expected": false
        }
      ]
    },
    {
      "name": "testDestroyIsland",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Wonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
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
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Demolish",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Demolish",
          "target": "Island"
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
          "name": "Elite Vanguard",
          "ability": "Flying",
          "expected": false
        }
      ]
    }
  ]
});
