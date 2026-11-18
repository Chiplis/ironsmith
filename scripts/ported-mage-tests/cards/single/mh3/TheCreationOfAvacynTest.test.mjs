import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/TheCreationOfAvacynTest.java",
  "tests": [
    {
      "name": "test_Creature_PutIntoPlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Avacyn, Angel of Hope"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 12
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Creature_DontPutIntoPlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Avacyn, Angel of Hope"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 12
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        }
      ]
    },
    {
      "name": "test_NotCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Helvault",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Helvault"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Helvault"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Helvault"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Helvault",
          "count": 1
        }
      ]
    },
    {
      "name": "test_StrionicResonator_DoubleCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Strionic Resonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Griselbrand",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Strionic Resonator",
          "ability": "{2}",
          "target": {
            "groups": [
              [
                "The Creation of Avacyn ability"
              ]
            ]
          }
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "optional": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Strionic Resonator",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 2
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Avacyn, Angel of Hope"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Griselbrand"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Griselbrand"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Griselbrand"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 4
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Griselbrand",
          "count": 1
        }
      ]
    },
    {
      "name": "test_StrionicResonator_CreatureAndPermanent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Strionic Resonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Helvault",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Strionic Resonator",
          "ability": "{2}",
          "target": {
            "groups": [
              [
                "The Creation of Avacyn ability"
              ]
            ]
          }
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "optional": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Strionic Resonator",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 2
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Avacyn, Angel of Hope"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Helvault"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Helvault"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Helvault"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 9
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Helvault",
          "count": 1
        }
      ]
    },
    {
      "name": "test_StrionicResonator_CreatureAndSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Strionic Resonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Creation of Avacyn",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Strionic Resonator",
          "ability": "{2}",
          "target": {
            "groups": [
              [
                "The Creation of Avacyn ability"
              ]
            ]
          }
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Choose new targets for the copy",
          "choices": [
            "No"
          ],
          "optional": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Strionic Resonator",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "count": 2
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Avacyn, Angel of Hope"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 card",
          "choices": [
            "Lightning Bolt"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Lightning Bolt"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Avacyn, Angel of Hope"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "exile",
            "player": 0,
            "name": "Lightning Bolt"
          },
          "count": 1,
          "checks": [
            {
              "property": "faceDown",
              "expected": false
            }
          ]
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 11
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    }
  ]
});
