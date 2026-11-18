import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/UnearthTest.java",
  "tests": [
    {
      "name": "testUnearthAttackExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hellspark Elemental",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Unearth"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Hellspark Elemental",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Hellspark Elemental",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hellspark Elemental",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Hellspark Elemental",
          "count": 1
        }
      ]
    },
    {
      "name": "testUndeadLeotau",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Undead Leotau",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Unearth"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Undead Leotau",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Undead Leotau",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Undead Leotau",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Undead Leotau",
          "count": 1
        }
      ]
    },
    {
      "name": "testUnearthWithPhasing",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Teferi's Veil",
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
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dregscape Zombie@zombie",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Dregscape Zombie",
          "ability": "Unearth",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Dregscape Zombie",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@zombie"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "@zombie",
          "defender": 1
        },
        {
          "op": "assertStackObject",
          "source": "Teferi's Veil",
          "count": 1,
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertStackObject",
          "source": "Teferi's Veil",
          "count": 1,
          "turn": 1,
          "phase": "END_COMBAT",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@zombie"
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@zombie",
          "count": 0
        },
        {
          "op": "assertStackObject",
          "source": "Dregscape Zombie",
          "count": 1,
          "turn": 1,
          "phase": "END_TURN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@zombie",
          "count": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@zombie"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@zombie",
          "count": 0
        }
      ]
    }
  ]
});
