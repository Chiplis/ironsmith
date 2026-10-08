import sys,json,datetime
from pathlib import Path
sys.dont_write_bytecode=True
root=Path.cwd();sys.path.insert(0,str(root/'scripts'))
import download_scryfall_cards as d
out=root/'reports/current-refresh-20261008/data'
now=lambda:datetime.datetime.now(datetime.timezone.utc).isoformat()
p={'started_at':now(),'metadata_source_url':d.BULK_DATA_URL}
m=d.fetch_json(d.BULK_DATA_URL)
(out/'bulk-metadata.json').write_text(json.dumps(m,indent=2)+'\n')
u=d.bulk_download_uri(m);raw=out/d.download_file_name(u)
d.download_file(u,raw)
p.update(raw_path=str(raw),raw_bytes=raw.stat().st_size,raw_sha256=d.file_sha256(raw),download_uri=u,source_updated_at=m.get('updated_at'))
filtered=out/'cards-current.json';total,kept=d.write_filtered_cards(raw,filtered)
d.write_download_metadata(m,out/'cards-current.metadata.json',cards_path=filtered,total=total,kept=kept)
p.update(filtered_path=str(filtered),filtered_bytes=filtered.stat().st_size,filtered_sha256=d.file_sha256(filtered),total_entries=total,kept_cards=kept,finished_at=now())
(out/'download-provenance.json').write_text(json.dumps(p,indent=2)+'\n');print(json.dumps(p,indent=2))
