import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mic/LyndeCheerfulTormentorTest.java",
  "tests": [
    {
      "name": "onlyBringsBackCursesFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lynde, Cheerful Tormentor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Bojuka Bog",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Bloodletting",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Disenchant",
          "target": "Curse of Bloodletting"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Bojuka Bog"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertGraveyardCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"at end step\", 2, PhaseStep.END_TURN, playerA, \"At the beginning of the next end step, return it to the battlefield attached to you\", 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN",
          "player": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Curse of Bloodletting",
          "count": 1
        }
      ]
    },
    {
      "name": "copyCardTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of the Pierced Heart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Clever Impersonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Lynde, Cheerful Tormentor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Claws of Gix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
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
          "name": "Curse of the Pierced Heart",
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"Turn 2 Upkeep\", 2, PhaseStep.PRECOMBAT_MAIN, playerB, 20 - 1)"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Clever Impersonator"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Curse of the Pierced Heart"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "playerA.getName()"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Curse of the Pierced Heart",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Curse of the Pierced Heart",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}, Sacrifice a permanent"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Curse of the Pierced Heart"
        },
        {
          "op": "unsupported",
          "source": "checkStackObject(\"After sac\", 2, PhaseStep.PRECOMBAT_MAIN, playerB, \"Whenever a Curse is put into your graveyard from the battlefield\", 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 2
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Curse of the Pierced Heart"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "At the beginning of enchanted"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Curse of the Pierced Heart",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Curse of the Pierced Heart",
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
          "life": 18
        }
      ]
    }
  ]
});
