import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m21/ArchfiendsVesselTest.java",
  "tests": [
    {
      "name": "enterFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Exhume",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "castSpell",
          "name": "Exhume",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Exhume",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Archfiend's Vessel"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Archfiend's Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 0
        }
      ]
    },
    {
      "name": "playFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Yawgmoth's Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Yawgmoth's Will",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Archfiend's Vessel"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 0
        }
      ]
    },
    {
      "name": "diesWithAbilityStack",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Exhume",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fatal Push",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "activateManaAbility",
          "source": "Swamp",
          "ability": "Add {B}",
          "player": 0,
          "count": 2,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Exhume",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Exhume",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Archfiend's Vessel"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Archfiend's Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "castSpell",
          "name": "Fatal Push",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Archfiend's Vessel"
        },
        {
          "op": "assertStackObject",
          "source": "Fatal Push",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Archfiend's Vessel"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Archfiend's Vessel",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Archfiend's Vessel",
          "count": 1
        }
      ]
    }
  ]
});
