import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/counterspell/ForceOfWillTest.java",
  "tests": [
    {
      "name": "testWithBlueCardsInHand",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thoughtseize",
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
          "zone": "HAND",
          "player": 1,
          "name": "Force of Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Remand",
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
          "name": "Thoughtseize",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Force of Will",
          "target": "Thoughtseize"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Cast with alternative cost: Pay 1 life, Exile a blue card from your hand (source: Force of Will"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Remand"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "CLEANUP"
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
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Remand",
          "count": 1
        }
      ]
    },
    {
      "name": "testWithRedCardsInHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thoughtseize",
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
          "zone": "HAND",
          "player": 1,
          "name": "Force of Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Fireball",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thoughtseize",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Force of Will",
          "target": "Thoughtseize"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "CLEANUP"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Force of Will$target=Thoughtseize\")) { Assert.fail(\"must throw error about not being able to cast Force of Will, but got:\\n\" + e.getMessage()); } }"
        }
      ]
    },
    {
      "name": "testWithLavinia",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lavinia, Azorius Renegade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
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
          "name": "Force of Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Remand",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Force of Will"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Cast with alternative cost: Pay 1 life, Exile a blue card from your hand (source: Force of Will"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Remand"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Force of Will",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Remand",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    },
    {
      "name": "testWithBoromir",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Boromir, Warden of the Tower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
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
          "name": "Force of Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Remand",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Force of Will"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Balduvian Bears"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Cast with alternative cost: Pay 1 life, Exile a blue card from your hand (source: Force of Will"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Remand"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Force of Will",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Remand",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    }
  ]
});
