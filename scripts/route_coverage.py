#!/usr/bin/env python3
"""Lists the REST routes of Discord's docs that `djs-rest` has no `Route` for.

Usage: python3 scripts/route_coverage.py [DOCS_DIR] [ROUTE_RS]
(defaults: .reference/discord-api-docs/developers, crates/djs-rest/src/route.rs)

Path parameters are compared by position only. Routes left out on purpose: D-105 (deprecated
pins, Modify Current User Nick), D-116 (widget.png), D-120 (Slack/GitHub webhooks) and Batch
Edit Application Command Permissions (disabled by Discord).
"""

import glob
import re
import sys


def normalize(path):
    path = re.sub(r"\[\\?\{([^}\\]+)\\?\}\]\([^)]*\)", "{x}", path)
    path = re.sub(r"\\?\{[^}]+\\?\}", "{x}", path)
    return path.replace("\\", "").strip()


def main():
    docs = sys.argv[1] if len(sys.argv) > 1 else ".reference/discord-api-docs/developers"
    route_rs = sys.argv[2] if len(sys.argv) > 2 else "crates/djs-rest/src/route.rs"
    documented = {}
    for name in glob.glob(docs + "/**/*.mdx", recursive=True):
        with open(name, encoding="utf-8") as file:
            for match in re.finditer(r'<Route method="(\w+)">(.*?)</Route>', file.read()):
                key = (match.group(1), normalize(match.group(2)))
                documented.setdefault(key, name.split("developers/")[-1])
    with open(route_rs, encoding="utf-8") as file:
        source = file.read()
    ours = {
        (match.group(1), re.sub(r"\{[^}]+\}", "{x}", match.group(2)))
        for match in re.finditer(r'=> (GET|POST|PUT|PATCH|DELETE) "([^"]+)"', source)
    }
    missing = sorted((page, key) for key, page in documented.items() if key not in ours)
    for page, (method, path) in missing:
        print(f"{page:45} {method:6} {path}")
    print(f"{len(documented)} documented, {len(ours)} routes, {len(missing)} missing")


if __name__ == "__main__":
    main()
