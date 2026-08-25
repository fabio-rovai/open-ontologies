"""Combine the pipeline's six artifacts (PARTS below) into the single object
ReplaySource, CompareSource and ValidationSource load: corpus, graph,
findings, chat, compare, validation."""
import argparse
import json
from pathlib import Path

PARTS = ("corpus", "graph", "findings", "chat", "compare", "validation")


def bundle(indir: Path) -> dict:
    out = {}
    for part in PARTS:
        path = indir / f"{part}.json"
        if not path.exists():
            raise SystemExit(f"missing artifact: {path}")
        out[part] = json.loads(path.read_text(encoding="utf-8"))
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--in", dest="indir", required=True, type=Path)
    ap.add_argument("--out", dest="outfile", required=True, type=Path)
    args = ap.parse_args()
    payload = bundle(args.indir)
    args.outfile.write_text(
        json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"bundled {len(payload['findings'])} findings into {args.outfile}")


if __name__ == "__main__":
    main()
