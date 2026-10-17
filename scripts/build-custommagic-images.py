#!/usr/bin/env python3
"""Build local full prints and art crops from manually reviewed image rectangles.
Usage: python3 scripts/build-custommagic-images.py /tmp/custommagic/images
Coordinates are fractions of the source (print) and isolated print (art).
No OCR, generative fill, or text reconstruction is used.
"""
import concurrent.futures
import argparse
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("images", type=Path)
parser.add_argument("--metadata-only", action="store_true", help="Refresh display metadata without re-encoding reviewed images")
args = parser.parse_args()
rows = json.loads((ROOT / "fixtures/custommagic/image-crops.json.fixture").read_text())
pool = json.loads((ROOT / "web/ui/public/custommagic/pool.json").read_text())
faces = {c["id"] + "-" + str(i): (f, c) for c in pool["cards"] for i, f in enumerate(c["draft"]["faces"])}
assert len({row["id"] for row in rows}) == len(rows), "Duplicate crop IDs"
assert {row["id"] for row in rows} == set(faces), "Every playable face must have a crop"
assert all(row.get("visuallyReviewed") for row in rows), "Review every crop before publishing"
compiled = json.loads((ROOT / "reports/custommagic/compilation.json").read_text())
types = {f["name"]: f["typeLine"] for c in compiled if c["status"] == "compiled" for f in c["preview"]["faces"]}
output = ROOT / "web/ui/public/custommagic/images"
output.mkdir(parents=True, exist_ok=True)
metadata = {}
def crop(image, box):
    w, h = image.size
    assert len(box) == 4 and 0 <= box[0] < box[2] <= 1 and 0 <= box[1] < box[3] <= 1
    return image.crop(tuple(round(v * (w if i % 2 == 0 else h)) for i, v in enumerate(box)))
def build(row):
    face, card = faces[row["id"]]
    paths = {variant: "custommagic/images/" + row["id"] + "-" + variant + ".webp" for variant in ["full", "art"]}
    if not args.metadata_only:
        original = ImageOps.exif_transpose(Image.open(args.images / (row["source"] + ".img"))).convert("RGB")
        full = crop(original, row["print"])
        art = crop(full, row["art"])
        if row.get("artRotate"):
            art = art.rotate(row["artRotate"], expand=True)
        if row.get("printRotate"):
            full = full.rotate(row["printRotate"], expand=True)
        # Keep small sources native; cap large scans for frontend bandwidth.
        full.thumbnail((1400, 1600), Image.Resampling.LANCZOS)
        art.thumbnail((1000, 1000), Image.Resampling.LANCZOS)
        paths = {}
        for variant, image in [("full", full), ("art", art)]:
            filename = row["id"] + "-" + variant + ".webp"
            image.save(output / filename, "WEBP", quality=94, method=4)
            paths[variant] = "custommagic/images/" + filename
    row["sourceSha256"] = hashlib.sha256((args.images / (row["source"] + ".img")).read_bytes()).hexdigest()
    # The only repeated face name is the two identical halves of Again.
    return face["name"].strip().lower(), {
        "name": face["name"],
        "typeLine": " ".join(face["supertypes"] + [t.removesuffix("s") if t == "Artifacts" else t for t in face["cardTypes"]]) + (" — " + " ".join(face["subtypes"]) if face["subtypes"] else ""),
        "engineTypeLine": types[face["name"]],
        **{key: face.get(key) for key in ["manaCost", "colorIndicator", "power", "toughness", "loyalty", "defense"]},
        "flavorText": card["source"]["post"], **paths,
    }
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
    for key, value in executor.map(build, rows):
        metadata.setdefault(key, value)
(ROOT / "web/ui/src/lib/custom-card-assets.generated.json").write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n")
(ROOT / "fixtures/custommagic/image-crops.json.fixture").write_text(json.dumps(rows, ensure_ascii=False, indent=2) + "\n")
print(f"Built {len(rows)} full prints and {len(rows)} art crops for {len(pool['cards'])} cards")
