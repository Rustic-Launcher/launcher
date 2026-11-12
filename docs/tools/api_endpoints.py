"""Regenerates the "## Endpoints" table of docs/API.md from services/api/openapi.json.

Usage: python docs/tools/api_endpoints.py [--check]
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = json.loads((ROOT / "services/api/openapi.json").read_text(encoding="utf-8"))
DOC = ROOT / "docs/API.md"
METHODS = ["get", "post", "put", "patch", "delete"]
# Group order = registration order in services/api/src/app.rs (openapi.json sorts paths).
ORDER = ["health", "auth", "sessions", "2fa", "me", "parental", "minecraft", "social", "parties",
         "messages", "moderation", "invitations", "notifications", "store", "cosmetics", "webhooks",
         "servers", "hosting", "content", "news", "companion", "gdpr", "admin"]


def table() -> str:
    groups: dict[str, list[tuple[str, str, str, str]]] = {}
    for path, item in SPEC["paths"].items():
        for method in METHODS:
            op = item.get(method)
            if not op:
                continue
            tag = (op.get("tags") or ["other"])[0]
            auth = "bearer" if op.get("security") else "public"
            groups.setdefault(tag, []).append((method.upper(), path, op.get("operationId", ""), auth))
    tags = [t for t in ORDER if t in groups] + sorted(t for t in groups if t not in ORDER)
    out = ["## Endpoints", "", "Generated from `openapi.json` (method, path, operationId = Rust client method name, auth).", ""]
    n = 0
    for tag in tags:
        out += [f"**{tag}**", "", "| Method | Path | operationId | Auth |", "|---|---|---|---|"]
        for m, p, o, a in sorted(groups[tag], key=lambda r: (r[1], r[0])):
            out.append(f"| {m} | `{p}` | `{o}` | {a} |")
            n += 1
        out.append("")
    out += [f"_{n} operations._", ""]
    return "\n".join(out)


def main() -> int:
    doc = DOC.read_text(encoding="utf-8")
    new = re.sub(r"## Endpoints\n.*?(?=\n## )", table(), doc, count=1, flags=re.S)
    if "--check" in sys.argv:
        if new != doc:
            print("docs/API.md endpoints table is stale: run python docs/tools/api_endpoints.py")
            return 1
        return 0
    DOC.write_text(new, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
