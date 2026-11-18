import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/GoblinGoliathTest.java",
  "tests": [
    {
      "name": "test_DoubleDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Goliath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barktooth Warbeard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "life": "20 - 3",
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{R}"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Goblin Goliath",
          "count": 1,
          "onTop": true,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "source": "Lightning Bolt",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Barktooth Warbeard"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "source": "Lightning Bolt",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "source": "Lightning Bolt",
          "count": 1,
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "assertDamageReceived",
          "player": 1,
          "name": "Barktooth Warbeard",
          "damage": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Barktooth Warbeard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 11
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Barktooth Warbeard",
          "count": 0
        }
      ]
    }
  ]
});
