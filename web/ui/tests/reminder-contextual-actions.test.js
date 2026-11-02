import test from 'node:test';
import assert from 'node:assert/strict';
import {expandStandaloneReminderActions} from '../src/lib/reminder-contextual-actions.js';
const reminder="(Transform this if it's not attached to a creature. Pay {2}{G} as a sorcery to unattach it.)";
test('standalone attachment reminders supply independently read executable instructions',()=>{
 const text=expandStandaloneReminderActions(`${reminder}\nEnchant creature you control\nMount's rider gets +2/+2.`,'Mount');
 assert.match(text,/If this permanent is not attached to a creature, transform it as a state-based action\./);
 assert.match(text,/\{2\}\{G\}: Unattach this permanent\. Activate only as a sorcery\./);
 assert.match(text,/Enchanted creature gets \+2\/\+2/);
});
test('unknown and inline explanatory reminders remain presentation',()=>{
 for(const original of ['An ability. (Pay {2} as a sorcery to unattach it.)', '(Transform this if it is not attached to a creature. Then draw a card.)',"Mount's rider gets +2/+2."]){assert.equal(expandStandaloneReminderActions(original,'Mount'),original);}
});
