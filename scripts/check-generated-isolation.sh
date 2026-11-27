#!/usr/bin/env bash
# Regression check: two copies of this crate with different generator inputs
# built into ONE target dir must not change each other's generated sources.
# Usage: scripts/check-generated-isolation.sh <target-dir> [work-dir]
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
target=$(realpath -m "${1:?target dir}")
work=$(realpath -m "${2:-$target/isolation-work}")
export CARGO_TARGET_DIR=$target
rm -rf "$work"; mkdir -p "$work"
for v in a b; do
  mkdir "$work/$v"
  tar -C "$root" --exclude=./target --exclude=./.git --exclude=./scripts -cf - . | tar -C "$work/$v" -xf -
done
# Different generator input in copy b: allow type index 982 again.
sed -i 's/ 982,//' "$work/b/codegen/generated-state-denylist.json"
# A different version gives copy b its own Cargo metadata hash, so its OUT_DIR
# differs from a's, as for two pins of the fork in one consumer target dir.
sed -i '0,/^version = "0.1.0"/s//version = "0.1.1"/' "$work/b/Cargo.toml"
cmp -s "$work/a/codegen/generated-state-denylist.json" "$work/b/codegen/generated-state-denylist.json" \
  && { echo "setup error: inputs identical"; exit 2; }

out_dir() { # <copy>: print the nw-network build script OUT_DIR of that copy
  (cd "$work/$1" && cargo build -p nw-network --message-format=json 2>/dev/null) |
    python3 -c '
import json,sys
o=None
for l in sys.stdin:
    try: m=json.loads(l)
    except ValueError: continue
    if m.get("reason")=="build-script-executed" and "/nw-network " in m["package_id"].replace("#"," /")+" " or (m.get("reason")=="build-script-executed" and m["package_id"].split("#")[-1].startswith("nw-network@")):
        o=m["out_dir"]
print(o)'
}
gen() { echo "$1/nw-network-generated/generated_states.rs"; }

oa=$(out_dir a); before=$(sha256sum "$(gen "$oa")" | cut -d' ' -f1)
ob=$(out_dir b)
after=$(sha256sum "$(gen "$oa")" | cut -d' ' -f1)
bsum=$(sha256sum "$(gen "$ob")" | cut -d' ' -f1)
echo "a OUT_DIR: $oa"; echo "b OUT_DIR: $ob"
echo "a before b build: $before"; echo "a after  b build: $after"; echo "b own:            $bsum"
[ "$oa" != "$ob" ] || { echo "setup error: same OUT_DIR"; exit 2; }
[ "$before" != "$bsum" ] || { echo "setup error: inputs generate identical code"; exit 2; }
if [ "$before" = "$after" ]; then echo "PASS: a's generated file unchanged"; else echo "FAIL: a's generated file was overwritten by b's build"; exit 1; fi
