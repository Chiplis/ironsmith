import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/DackFaydenTest.java",
  "tests": [
    {
      "name": "testDackFaydenEmblem",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dack Fayden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter@art",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gut Shot",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 4,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 5,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 6,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 7,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 7,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 7,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "-6: You get an emblem",
          "player": 0,
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 9,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 1,
          "counter": "loyalty"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 1 of Gut Shot",
          "choices": [
            "2 life (Phyrexian)"
          ]
        },
        {
          "op": "castSpell",
          "name": "Gut Shot",
          "player": 0,
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "target": "@art"
        },
        {
          "op": "assertStackObject",
          "source": "Gut Shot",
          "count": 1,
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Ornithopter"
        },
        {
          "op": "assertStackObject",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "text": "Whenever you cast a spell that targets one or more permanents",
          "count": 1,
          "abilityKind": "Triggered",
          "source": "Emblem",
          "controller": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@art"
          },
          "count": 1,
          "checks": []
        }
      ]
    },
    {
      "name": "testDackFaydenEmblemAcrossZones",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dack Fayden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter@art",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gut Shot",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unsummon",
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
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 4,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 5,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 6,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "+1: Target player draws",
          "player": 0,
          "turn": 7,
          "phase": "PRECOMBAT_MAIN",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 7,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 7,
          "counter": "loyalty"
        },
        {
          "op": "activateAbility",
          "source": "Dack Fayden",
          "ability": "-6: You get an emblem",
          "player": 0,
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Dack Fayden",
          "count": 1,
          "turn": 9,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 9,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Dack Fayden",
          "count": 1,
          "counter": "loyalty"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "hybrid_payment",
          "description": "Choose how to pay pip 1 of Gut Shot",
          "choices": [
            "2 life (Phyrexian)"
          ]
        },
        {
          "op": "castSpell",
          "name": "Gut Shot",
          "player": 0,
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "target": "@art"
        },
        {
          "op": "assertStackObject",
          "source": "Gut Shot",
          "count": 1,
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Ornithopter"
        },
        {
          "op": "assertStackObject",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "text": "Whenever you cast a spell that targets one or more permanents",
          "count": 1,
          "abilityKind": "Triggered",
          "source": "Emblem",
          "controller": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@art"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "castSpell",
          "name": "Unsummon",
          "player": 0,
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "target": "@art"
        },
        {
          "op": "assertStackObject",
          "source": "Unsummon",
          "count": 1,
          "turn": 10,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Ornithopter"
        },
        {
          "op": "waitStackResolved",
          "turn": 10,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "@art",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Ornithopter",
          "player": 1,
          "turn": 10,
          "phase": "POSTCOMBAT_MAIN",
          "objectAlias": "@art"
        },
        {
          "op": "assertStackObject",
          "source": "Ornithopter",
          "count": 1,
          "turn": 10,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 10,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@art"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@art"
          },
          "count": 0,
          "checks": []
        }
      ]
    }
  ]
});
