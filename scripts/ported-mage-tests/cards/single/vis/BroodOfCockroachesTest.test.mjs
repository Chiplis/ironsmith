import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/vis/BroodOfCockroachesTest.java",
  "tests": [
    {
      "name": "should_reduce_life_of_playerA_by_1_at_the_beginning_of_the_next_end_step",
      "operations": [
        {
          "op": "setLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "unsupported",
          "source": "playerA_casts_Brood_of_Cockroaches_at_precombat_main_phase()"
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
          "life": 16
        }
      ]
    },
    {
      "name": "should_not_reduce_life_of_playerA_by_1_at_post_combat_main_step",
      "operations": [
        {
          "op": "setLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "unsupported",
          "source": "playerA_casts_Brood_of_Cockroaches_at_precombat_main_phase()"
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
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "should_return_Brood_of_Cockroaches_to_playerA_hand_end_of_turn",
      "operations": [
        {
          "op": "setLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "unsupported",
          "source": "playerA_casts_Brood_of_Cockroaches_at_precombat_main_phase()"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Brood of Cockroaches",
          "count": 1
        }
      ]
    },
    {
      "name": "should_not_return_Brood_of_Cockroaches_to_playerA_at_post_combat_step",
      "operations": [
        {
          "op": "setLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "unsupported",
          "source": "playerA_casts_Brood_of_Cockroaches_at_precombat_main_phase()"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Brood of Cockroaches",
          "count": 0
        }
      ]
    }
  ]
});
