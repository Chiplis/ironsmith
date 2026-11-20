import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/RosheenMeandererManaXTest.java",
  "tests": [
    {
      "name": "test_SimpleKicker",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose optional costs for Verdeloth the Ancient",
          "player": 0,
          "choices": [
            "Kicker: 0"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "holdStack": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Verdeloth the Ancient",
          "isSpell": true,
          "count": 1,
          "xValue": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Saproling Token",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_KickerWithXMana",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rosheen Meanderer",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {C}",
          "source": "Rosheen Meanderer"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": 4
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose optional costs for Verdeloth the Ancient",
          "player": 0,
          "choices": [
            "Kicker: 0"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "holdStack": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Verdeloth the Ancient",
          "isSpell": true,
          "count": 1,
          "xValue": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Saproling Token",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_KickerWithXZero",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rosheen Meanderer",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Add {C}",
          "source": "Rosheen Meanderer"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": 4
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "description": "Choose optional costs for Verdeloth the Ancient",
          "player": 0,
          "choices": [
            "Kicker: 0"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "holdStack": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=0"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Verdeloth the Ancient",
          "isSpell": true,
          "count": 1,
          "xValue": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Verdeloth the Ancient",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "END_TURN",
          "player": 0,
          "name": "Saproling Token",
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_SimpleDragon",
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
          "name": "Flameblast Dragon",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Flameblast Dragon",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "number",
          "descriptionPrefix": "Choose value for X",
          "value": 3
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "life": "20 - 5 - 3",
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_PayXForDragonAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flameblast Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rosheen Meanderer",
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Flameblast Dragon",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{T}: Add {C}",
          "source": "Rosheen Meanderer"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "C",
          "amount": 4,
          "holdStack": true,
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "seatIndependent": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "source": "Flameblast Dragon",
          "onTop": true,
          "count": 1,
          "abilityKind": "Triggered",
          "targetCount": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertChoiceMenu",
          "kind": "number",
          "player": 0,
          "value": 3,
          "descriptionPrefix": "Choose value for X"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "resolutions": 1
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "life": "20 - 5 - 3",
          "seatIndependent": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_PayXForCondescendPrevent",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Condescend",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rosheen Meanderer",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Abandon Hope",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Condescend"
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "descriptionPrefix": "Choose value for X",
          "value": 2
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Lightning Bolt"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "C",
          "amount": "4",
          "holdStack": true,
          "note": "XMage checkManaPool reads the pool while Condescend is still on the stack; without holdStack the harness resolves Condescend first and the {X}=2 payment leaves 2."
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Abandon Hope"
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "life": "20 - 3",
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Condescend",
          "count": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
