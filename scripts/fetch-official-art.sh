#!/usr/bin/env bash
# Downloads the official card images (© Nerdlab Games) from the fan database
# github.com/ryanascherr/mindbug into web/public/official/ (git-ignored),
# then writes manifest.json: normalized card name → image.
# PERSONAL USE ONLY: never commit or publish these files.
set -euo pipefail
cd "$(dirname "$0")/.."

REPO="ryanascherr/mindbug"
OUT="web/public/official"
mkdir -p "$OUT/cards" "$OUT/mindbugs"

TREE=$(mktemp)
trap 'rm -f "$TREE"' EXIT
curl -fsSL "https://api.github.com/repos/$REPO/git/trees/HEAD?recursive=1" -o "$TREE"

python3 - "$TREE" <<'EOF' |
import json, sys
tree = json.load(open(sys.argv[1]))["tree"]
for t in tree:
    p = t["path"]
    if t["type"] == "blob" and p.startswith(("img/cards/", "img/mindbugs/")) and p.lower().endswith((".jpg", ".png", ".webp")):
        print(p)
EOF
  xargs -P 4 -I {} sh -c '
    dest="'"$OUT"'/$(echo "{}" | sed "s#^img/##")"
    [ -s "$dest" ] || curl -fsSL "https://raw.githubusercontent.com/'"$REPO"'/HEAD/{}" -o "$dest"
  '

python3 - "$OUT" <<'EOF'
import json, os, re, sys
out = sys.argv[1]
norm = lambda s: re.sub(r"[^a-z0-9]", "", s.lower())
cards = {norm(os.path.splitext(f)[0]): f"cards/{f}" for f in sorted(os.listdir(f"{out}/cards"))}
mindbugs = sorted(f"mindbugs/{f}" for f in os.listdir(f"{out}/mindbugs"))
json.dump({"cards": cards, "mindbugs": mindbugs}, open(f"{out}/manifest.json", "w"), indent=1)
print(f"{len(cards)} card images, {len(mindbugs)} Mindbug images → {out}/manifest.json")
EOF
