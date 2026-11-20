import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/PeterParkerTest.java",
  "tests": [
    {
      "name": "testAmazingSpiderMan",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tap all creatures",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Tap all Bears."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Peter Parker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chainer, Nightmare Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Absolute Virtue",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 8
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Amazing Spider-Man"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "tap all creatures",
          "ability": "Tap all Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "tap all creatures",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "tapped": true,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "methods": [
            "normal",
            "play_from"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner with Web-slinging"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Adelbert Steiner"
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Chainer, Nightmare Adept",
          "ability": "Discard a card"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Adelbert Steiner"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Chainer, Nightmare Adept",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "methods": [
            "normal"
          ]
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Virtue",
          "zone": "graveyard",
          "methods": [
            "alternative_price",
            "play_from"
          ],
          "note": "Chainer supplies the graveyard permission; web-slinging is a separate alternative cost for that cast (CR 118.9, 601.2b), which Ironsmith names alternative_price."
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Virtue",
          "zone": "graveyard",
          "filterMethod": "Web-slinging",
          "methods": [
            "alternative_price"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Virtue with Web-slinging"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Absolute Virtue"
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Virtue",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Absolute Virtue",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spider Token",
          "count": 0,
          "seatIndependent": true
        }
      ]
    },
    {
      "name": "testTransform",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tap all creatures",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Tap all Bears."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Peter Parker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "tap all"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Peter Parker",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{G}{W}{U}: Transform"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner with Web-slinging"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "testTransformLosesWebSlinging",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tap all creatures",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Tap all Bears."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Peter Parker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Face Switch Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Transform target permanent."
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "tap all creatures",
          "ability": "Tap all Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "tap all creatures",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Peter Parker"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Peter Parker"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Peter Parker",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spider Token",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Peter Parker",
          "ability": "Transform Peter Parker"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Peter Parker",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "methods": [
            "normal",
            "play_from"
          ]
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Face Switch Fixture",
          "ability": "Transform target permanent",
          "target": "Amazing Spider-Man"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Face Switch Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Amazing Spider-Man"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Peter Parker",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "methods": [
            "normal"
          ]
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spider Token",
          "count": 1,
          "seatIndependent": true
        }
      ]
    },
    {
      "name": "testTransformCastSecondSideDoesntTriggerFront",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tap all creatures",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Tap all Bears."
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Peter Parker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tropical Island",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Adelbert Steiner",
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
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Face Switch Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Transform target permanent."
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "tap all creatures",
          "ability": "Tap all Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "tap all creatures",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Amazing Spider-Man"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Spider Token",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "methods": [
            "normal",
            "play_from"
          ]
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Face Switch Fixture",
          "ability": "Transform target permanent",
          "target": "Amazing Spider-Man"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Face Switch Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Amazing Spider-Man"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Peter Parker",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "methods": [
            "normal"
          ]
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "Peter Parker"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Unsummon",
          "objectTarget": "Peter Parker"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Peter Parker",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Amazing Spider-Man"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Amazing Spider-Man",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Peter Parker",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Spider Token",
          "count": 0,
          "seatIndependent": true
        }
      ]
    }
  ]
});
