#!/usr/bin/env python3
import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description="Write distribution-readiness proof JSON.")
    parser.add_argument("output_path", help="Destination JSON path")
    parser.add_argument("version", help="Release version without the leading v")
    parser.add_argument("--repo-slug", default="pietroMastro92/Sbobino")
    parser.add_argument("--commit-sha", default="")
    args = parser.parse_args()

    output_path = Path(args.output_path).resolve()
    payload = {
        "schema_version": 1,
        "version": args.version.strip(),
        "release_tag": f"v{args.version.strip()}",
        "repo_slug": args.repo_slug.strip(),
        "commit_sha": args.commit_sha.strip(),
        "status": "passed",
        "gate": "distribution_readiness.sh",
        "generated_at_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    }
    output_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
