#!/usr/bin/env python3
"""Collect the all-time top 1,000 posts for a separate visual review.

This collects sources, not transcriptions. A 403/CAPTCHA is a hard stop.
Usage: python3 scripts/scrape-custommagic.py --out /tmp/custommagic
"""
import argparse
import html
import json
import time
import urllib.parse
import urllib.request
from pathlib import Path


def image_urls(post):
    images = []
    metadata = post.get("media_metadata", {})
    for item in post.get("gallery_data", {}).get("items", []):
        source = metadata.get(item["media_id"], {}).get("s", {})
        if source.get("u"):
            images.append(html.unescape(source["u"]))
    if not images:
        url = post.get("url_overridden_by_dest", post.get("url", ""))
        if urllib.parse.urlparse(url).path.lower().endswith((".png", ".jpg", ".jpeg", ".webp", ".gif")):
            images.append(html.unescape(url))
        else:
            for image in post.get("preview", {}).get("images", []):
                images.append(html.unescape(image["source"]["url"]))
    return list(dict.fromkeys(images))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    posts, seen, after = [], set(), None
    for page in range(10):
        query = {"t": "all", "limit": 100, "raw_json": 1}
        if after:
            query["after"] = after
        url = "https://www.reddit.com/r/custommagic/top.json?" + urllib.parse.urlencode(query)
        request = urllib.request.Request(url, headers={"User-Agent": "IronsmithCustomCardResearch/1.0"})
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.load(response)
        (args.out / f"page-{page + 1:02}.json").write_text(json.dumps(payload, indent=2))
        for child in payload["data"]["children"]:
            post = child["data"]
            if post["id"] in seen or post.get("stickied"):
                continue
            seen.add(post["id"])
            posts.append({
                "rank": len(posts) + 1, "id": post["id"], "title": post["title"],
                "author": post.get("author"), "score": post.get("score"),
                "permalink": "https://www.reddit.com" + post["permalink"],
                "url": post.get("url"), "images": image_urls(post),
            })
        manifest = {"subreddit": "custommagic", "ranking": "top/all", "requestedPosts": 1000,
                    "fetchedAt": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "posts": posts}
        (args.out / "posts.json").write_text(json.dumps(manifest, indent=2) + "\n")
        after = payload["data"].get("after")
        print(f"Collected {len(posts)} posts", flush=True)
        if not after or len(posts) >= 1000:
            break
        time.sleep(2)
    if len(posts) != 1000:
        raise SystemExit(f"Incomplete listing: {len(posts)}/1000 posts; do not label this a complete audit")


if __name__ == "__main__":
    main()
