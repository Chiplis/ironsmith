import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dom/CorrosiveOozeTest.java",
  "tests": [
    {
      "name": "testTradeAttacking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Fugitive Wizard",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Fugitive Wizard"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Fugitive Wizard",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Corrosive Ooze",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Fugitive Wizard",
          "attacker": "Corrosive Ooze"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testTradeBlocking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Fugitive Wizard",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Fugitive Wizard"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Fugitive Wizard",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Fugitive Wizard",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Corrosive Ooze",
          "attacker": "Fugitive Wizard"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fugitive Wizard",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testBounceAttacking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Kraken Hatchling",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Kraken Hatchling"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Kraken Hatchling",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Corrosive Ooze",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Kraken Hatchling",
          "attacker": "Corrosive Ooze"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testBounceBlocking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Kraken Hatchling",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Kraken Hatchling"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Kraken Hatchling",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Kraken Hatchling",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Corrosive Ooze",
          "attacker": "Kraken Hatchling"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testEatenAttacking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Centaur Courser",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Centaur Courser"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Centaur Courser",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Corrosive Ooze",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Centaur Courser",
          "attacker": "Corrosive Ooze"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testEatenBlocking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Centaur Courser",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Centaur Courser"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Centaur Courser",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Centaur Courser",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Corrosive Ooze",
          "attacker": "Centaur Courser"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testEatAttacking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Acolyte of Xathrid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Acolyte of Xathrid",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Acolyte of Xathrid"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Acolyte of Xathrid",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Corrosive Ooze",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Acolyte of Xathrid",
          "attacker": "Corrosive Ooze"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Acolyte of Xathrid",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    },
    {
      "name": "testEatBlocking",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Acolyte of Xathrid",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Equip",
          "target": "Acolyte of Xathrid",
          "source": "Short Sword"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "source": "Short Sword",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Acolyte of Xathrid"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 1,
          "attachment": "Short Sword",
          "target": "Acolyte of Xathrid",
          "expected": true
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Acolyte of Xathrid",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Corrosive Ooze",
          "attacker": "Acolyte of Xathrid"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "END_COMBAT",
          "source": "Corrosive Ooze",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_COMBAT",
          "resolutions": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Corrosive Ooze",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Acolyte of Xathrid",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Short Sword",
          "count": 1
        }
      ]
    }
  ]
});
