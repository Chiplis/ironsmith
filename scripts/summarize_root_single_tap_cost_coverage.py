#!/usr/bin/env python3
"""Preserve exact path mappings for root's reviewed special single-object tap cases."""
import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];REPORTS=ROOT/'reports/runtime-audit'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 inventory=json.loads((REPORTS/'choose-consume-cost-candidates.json').read_text());candidates={(r['card'],r['path']):r for r in inventory['rows'] if r['count_shape']=='single'and r['consumer_kind']=='TapEffect'};rows=[];seen=set()
 for name in ['single-land-tap-reviewed-classification.json','single-special-seven-reviewed-classification.json']:
  file=REPORTS/name;review=json.loads(file.read_text());raw=REPORTS/review['source_report'];assert digest(raw)==review['source_sha256'];data=json.loads(raw.read_text())
  for entry in review['path_coverage']:
   key=(entry['card'],entry['path']);assert key not in seen;seen.add(key);candidate=candidates[key];assert candidate['consumer_path']==entry['consumer_path']
   for i in entry['source_rows']:assert data['rows'][i]['card']==entry['card']
   rows.append(dict(candidate,**{k:v for k,v in entry.items()if k not in candidate},reviewed_report=str(file.relative_to(ROOT)),reviewed_sha256=digest(file),source_report={'path':str(raw.relative_to(ROOT)),'sha256':digest(raw)}))
 output=dict(scope=__doc__,rows=rows,path_count=len(rows),generator_sha256=digest(Path(__file__)),limitations=['Only the explicit activation paths and scenario scopes are covered, not whole cards.','Earlier legality failures and availability diagnostics never imply that effects executed.'])
 (REPORTS/'root-single-tap-cost-path-coverage.json').write_text(json.dumps(output,indent=2)+'\n');print(json.dumps({'reviewed_paths':len(rows)}))
if __name__=='__main__':main()
