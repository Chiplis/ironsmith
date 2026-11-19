import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/WorldgorgerDragonTest.java",
  "tests": [
    {
      "name": "testDisabledEffectOnChangeZone",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gerrard's Battle Cry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Worldgorger Dragon"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Terror",
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
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
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Terror",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gerrard's Battle Cry",
          "count": 1
        }
      ]
    },
    {
      "name": "testWithAnimateDead",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Animate Dead",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Volcanic Geyser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Staunch Defenders",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animate Dead",
          "target": "Worldgorger Dragon"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "objectTarget": "Worldgorger Dragon",
          "targetCount": 1
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 40
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "R",
          "amount": 20
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Volcanic Geyser",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=20"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Volcanic Geyser",
          "count": 1,
          "onTop": true,
          "xValue": 20
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
          "name": "Volcanic Geyser",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 0
        },
        {
          "op": "assertResult",
          "player": 0,
          "result": "WON"
        }
      ]
    },
    {
      "name": "testWithAnimateDeadDifferentTargets",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Animate Dead",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Volcanic Geyser",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Staunch Defenders",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Animate Dead",
          "target": "Worldgorger Dragon"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "objectTarget": "Worldgorger Dragon",
          "targetCount": 1
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Worldgorger Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Worldgorger Dragon",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "exile all other permanents"
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
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "sacrifice"
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
          "source": "Worldgorger Dragon",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "leaves the battlefield"
        },
        {
          "op": "assertChoiceMenu",
          "kind": "select_objects",
          "player": 0,
          "description": "Attach Aura to",
          "options": [
            "Silvercoat Lion",
            "Worldgorger Dragon"
          ],
          "min": 1,
          "max": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Animate Dead"
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
          "source": "Staunch Defenders",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "gain 4 life"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "{T}: Add {R}",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Animate Dead",
          "count": 1,
          "onTop": true,
          "abilityKind": "Triggered",
          "text": "return enchanted creature"
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Animate Dead",
          "target": "Silvercoat Lion",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 28
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "R",
          "amount": 9
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Volcanic Geyser",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=9"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Volcanic Geyser",
          "count": 1,
          "onTop": true,
          "xValue": 9
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
          "name": "Volcanic Geyser",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 11
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Worldgorger Dragon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRGC",
          "amount": 0
        },
        {
          "op": "assertManaOptions",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "options": [
            ""
          ],
          "exact": true
        }
      ]
    }
  ]
});
