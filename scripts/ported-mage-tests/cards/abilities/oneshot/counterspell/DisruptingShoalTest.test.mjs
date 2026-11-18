import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/counterspell/DisruptingShoalTest.java",
  "tests": [
    {
      "name": "testWithManaPaymentEqual",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disrupting Shoal",
          "target": "Silvercoat Lion",
          "actionLabel": "Cast Disrupting Shoal"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose casting method for Disrupting Shoal",
          "choices": [
            "Normal: {X}{U}{U}"
          ],
          "optional": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "description": "Choose value for X",
          "value": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Disrupting Shoal",
          "count": 1,
          "xValue": 2,
          "objectTarget": "Silvercoat Lion"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testWithManaPaymentDifferent",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disrupting Shoal",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disrupting Shoal",
          "target": "Silvercoat Lion",
          "actionLabel": "Cast Disrupting Shoal"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose casting method for Disrupting Shoal",
          "choices": [
            "Normal: {X}{U}{U}"
          ],
          "optional": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "description": "Choose value for X",
          "value": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Disrupting Shoal",
          "count": 1,
          "xValue": 1,
          "objectTarget": "Silvercoat Lion"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testWithBlueCardsInHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spell Snare",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mistfire Adept",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pillarfield Ox"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disrupting Shoal",
          "target": "Pillarfield Ox",
          "actionLabel": "Cast Disrupting Shoal (parsed alternative cost)"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose casting method for Disrupting Shoal",
          "choices": [
            "Parsed alternative cost: Exile a blue card with a constrained mana value from your hand, Exile those cards"
          ],
          "optional": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "description": "Choose value for X",
          "value": 4
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Mistfire Adept"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Disrupting Shoal",
          "count": 1,
          "xValue": 4,
          "objectTarget": "Pillarfield Ox"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Spell Snare",
          "expected": false,
          "holdStack": true
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
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "count": 1,
          "name": "Mistfire Adept"
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Mistfire Adept",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Spell Snare",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        }
      ]
    },
    {
      "name": "testWithFuseCardCounterCMC_LeftIgnore",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
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
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Far // Away",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disrupting Shoal",
          "target": "Grizzly Bears",
          "actionLabel": "Cast Disrupting Shoal (parsed alternative cost)"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose casting method for Disrupting Shoal",
          "choices": [
            "Parsed alternative cost: Exile a blue card with a constrained mana value from your hand, Exile those cards"
          ],
          "optional": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "description": "Choose value for X",
          "value": 5
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Far // Away"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Disrupting Shoal",
          "count": 1,
          "xValue": 5,
          "objectTarget": "Grizzly Bears"
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
          "op": "assertExileCount",
          "player": 1,
          "count": 1,
          "name": "Far // Away"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        }
      ]
    },
    {
      "name": "testWithFuseCardCounterCMC_RightIgnore",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Far // Away",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Centaur Courser"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disrupting Shoal",
          "target": "Centaur Courser",
          "actionLabel": "Cast Disrupting Shoal (parsed alternative cost)"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose casting method for Disrupting Shoal",
          "choices": [
            "Parsed alternative cost: Exile a blue card with a constrained mana value from your hand, Exile those cards"
          ],
          "optional": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "description": "Choose value for X",
          "value": 5
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Far // Away"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Disrupting Shoal",
          "count": 1,
          "xValue": 5,
          "objectTarget": "Centaur Courser"
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
          "op": "assertExileCount",
          "player": 1,
          "count": 1,
          "name": "Far // Away"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Centaur Courser",
          "count": 0
        }
      ]
    },
    {
      "name": "testWithFuseCardShouldNotCounterCMC_BothUses",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Far // Away",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Air Elemental"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disrupting Shoal",
          "target": "Air Elemental",
          "actionLabel": "Cast Disrupting Shoal (parsed alternative cost)"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "options",
          "description": "Choose casting method for Disrupting Shoal",
          "choices": [
            "Parsed alternative cost: Exile a blue card with a constrained mana value from your hand, Exile those cards"
          ],
          "optional": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 1,
          "kind": "number",
          "description": "Choose value for X",
          "value": 5
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Far // Away"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Disrupting Shoal",
          "count": 1,
          "xValue": 5,
          "objectTarget": "Air Elemental"
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
          "op": "assertExileCount",
          "player": 1,
          "count": 1,
          "name": "Far // Away"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Disrupting Shoal",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Air Elemental",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        }
      ]
    }
  ]
});
