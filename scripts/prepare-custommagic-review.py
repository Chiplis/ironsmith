#!/usr/bin/env python3
"""Normalize browser-saved public listing pages and cache images for visual review."""
import argparse
import concurrent.futures
import importlib.util
import json
import time
import urllib.request
import urllib.parse
from pathlib import Path

spec = importlib.util.spec_from_file_location("scraper", Path(__file__).with_name("scrape-custommagic.py"))
scraper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scraper)

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
args = parser.parse_args()
posts, seen = [], set()
for page in sorted(args.directory.glob("page-*.json")):
    data = json.loads(page.read_text())["data"]
    for item in data["children"]:
        post = item["data"]
        if post["id"] in seen or post.get("stickied"):
            continue
        seen.add(post["id"])
        posts.append({"rank": len(posts) + 1, "id": post["id"], "title": post["title"],
                      "author": post.get("author"), "score": post.get("score"),
                      "permalink": "https://www.reddit.com" + post["permalink"],
                      "url": post.get("url"), "images": scraper.image_urls(post)})
manifest = {"subreddit": "custommagic", "ranking": "top/all", "requestedPosts": 1000,
            "fetchedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "posts": posts}
(args.directory / "posts.json").write_text(json.dumps(manifest, indent=2) + "\n")
image_dir = args.directory / "images"
image_dir.mkdir(exist_ok=True)

def download(item):
    rank, index, url = item
    path = image_dir / f"{rank:04}-{index:02}.img"
    if path.exists():
        return
    try:
        request = urllib.request.Request(url, headers={"User-Agent": "IronsmithCustomCardResearch/1.0"})
        with urllib.request.urlopen(request, timeout=30) as response:
            if not response.headers.get("Content-Type", "").startswith("image/"):
                raise ValueError("Not an image")
            path.write_bytes(response.read())
    except Exception as error:
        if urllib.parse.urlparse(url).hostname == "external-preview.redd.it":
            original = "https://i.redd.it" + urllib.parse.urlparse(url).path
            try:
                with urllib.request.urlopen(original, timeout=30) as response:
                    if not response.headers.get("Content-Type", "").startswith("image/"):
                        raise ValueError("Not an image")
                    path.write_bytes(response.read())
                return
            except Exception as fallback_error:
                error = f"{error}; original image: {fallback_error}"
        return {"rank": rank, "index": index, "url": url, "error": str(error)}

jobs = [(post["rank"], index, url) for post in posts for index, url in enumerate(post["images"])]
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
    failures = [error for error in executor.map(download, jobs) if error]
(args.directory / "image-errors.json").write_text(json.dumps(failures, indent=2) + "\n")
print(f"{len(posts)} posts; {len(jobs)} images; {len(failures)} download failures")
