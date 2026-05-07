import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ths/AshiokNightmareWeaverTest.java",
  "tests": [
    {
      "name": "testFirstAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashiok, Nightmare Weaver",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: Exile the top three cards of target opponent's library.",
          "target": 1
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ashiok, Nightmare Weaver",
          "counter": "LOYALTY",
          "count": 5
        }
      ]
    },
    {
      "name": "testSecondAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ashiok, Nightmare Weaver",
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
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Prophet of Kruphix",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+2: Exile the top three cards of target opponent's library.",
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-X: Put a creature card with mana value X exiled with {this} onto the battlefield under your control. That creature is a Nightmare in addition to its other types."
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=5"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Prophet of Kruphix"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
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
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Prophet of Kruphix",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ashiok, Nightmare Weaver",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Mountain\", false)"
        }
      ]
    }
  ]
});
