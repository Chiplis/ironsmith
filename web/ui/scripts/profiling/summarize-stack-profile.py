import collections
import json
import math
from pathlib import Path
import statistics
import sys

root = Path(sys.argv[1])
samples = json.loads((root / 'samples.json').read_text())
events = json.loads((root / 'trace.json').read_text())['traceEvents'] if (root / 'trace.json').exists() else []
frames = samples['frames']
idle = [x['delta'] for x in frames if x['phase'] in ('idle', 'final-idle')]
interval = statistics.median(idle)
groups = collections.defaultdict(list)
for row in frames:
    groups[row['phase']].append(row['delta'])

def percentile(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs)-1, math.ceil(len(xs)*p)-1)]

summary = {'baseline_interval_ms': interval, 'phases': {}, 'errors': samples['errors']}
for phase, values in groups.items():
    if phase.startswith('capture-'):
        continue
    summary['phases'][phase] = {
        'frames': len(values), 'duration_ms': sum(values),
        'p50_ms': statistics.median(values), 'p95_ms': percentile(values, .95),
        'p99_ms': percentile(values, .99), 'max_ms': max(values),
        'gaps_over_1_5_refresh': sum(x > interval*1.5 for x in values),
        'gaps_over_50ms': sum(x > 50 for x in values),
        'estimated_missed_refreshes': sum(max(0, round(x/interval)-1) for x in values),
    }
    callbacks = [row['callbackDelta'] for row in frames if row['phase'] == phase and 'callbackDelta' in row]
    if callbacks:
        summary['phases'][phase].update({
            'callback_p95_ms': percentile(callbacks, .95),
            'callback_max_ms': max(callbacks),
            'callback_gaps_over_50ms': sum(value > 50 for value in callbacks),
        })

marks = sorted((e for e in events if e.get('cat') == 'blink.user_timing' and e.get('name') in groups and e.get('ph') in ('I', 'R')), key=lambda e: e['ts'])
main_threads = {(e['pid'], e['tid']) for e in events if e.get('name') == 'thread_name' and e.get('args', {}).get('name') == 'CrRendererMain'}
trace_summary = collections.defaultdict(lambda: collections.defaultdict(lambda: {'count': 0, 'total_ms': 0, 'max_ms': 0}))
dropped = collections.defaultdict(lambda: {'all': 0, 'partial_updates': 0})
for i, mark in enumerate(marks):
    end = marks[i+1]['ts'] if i+1 < len(marks) else math.inf
    drops = [e for e in events if e['name'] == 'DroppedFrame' and e['pid'] == mark['pid'] and mark['ts'] <= e.get('ts', -1) < end]
    dropped[mark['name']]['all'] += len(drops)
    dropped[mark['name']]['partial_updates'] += sum(bool(e.get('args', {}).get('hasPartialUpdate')) for e in drops)
    window = [e for e in events if mark['ts'] <= e.get('ts', -1) < end and e.get('ph') == 'X' and (e['pid'], e['tid']) in main_threads]
    totals = trace_summary[mark['name']]
    for e in window:
        name, ms = e['name'], e.get('dur', 0)/1000
        if name in ('Layout', 'UpdateLayoutTree', 'Paint', 'PrePaint', 'FunctionCall', 'EvaluateScript', 'FireAnimationFrame', 'RunTask', 'HitTest'):
            totals[name]['count'] += 1
            totals[name]['total_ms'] += ms
            totals[name]['max_ms'] = max(totals[name]['max_ms'], ms)
summary['trace_main_thread'] = {phase: dict(totals) for phase, totals in trace_summary.items() if not phase.startswith('capture-')}
summary['chrome_dropped_frame_events'] = {phase: counts for phase, counts in dropped.items() if not phase.startswith('capture-')}
summary['long_tasks'] = samples['longTasks']
summary['workers'] = samples.get('workers', [])
summary['long_animation_frames'] = samples['longAnimationFrames']
summary['trace_event_counts'] = collections.Counter(e['name'] for e in events).most_common(35)
(root / 'summary.json').write_text(json.dumps(summary, indent=2)+'\n')
for phase, data in summary['phases'].items():
    print(phase, json.dumps({k: round(v, 2) for k, v in data.items()}))
print('long tasks:', len(samples['longTasks']), 'long animation frames:', len(samples['longAnimationFrames']))
print('errors:', samples['errors'])
