#!/usr/bin/env bash
set -euo pipefail

SOURCE_ICON="${1:-icon.png}"
OUTPUT_ICON="${2:-packaging/macos/WebNotes.icns}"
ICONSET_DIR="$(dirname "${OUTPUT_ICON}")/WebNotes.iconset"

if [[ ! -f "${SOURCE_ICON}" ]]; then
  echo "Missing source icon: ${SOURCE_ICON}" >&2
  exit 1
fi

rm -rf "${ICONSET_DIR}"
mkdir -p "${ICONSET_DIR}" "$(dirname "${OUTPUT_ICON}")"

sips -z 16 16 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_16x16.png" >/dev/null
sips -z 32 32 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_32x32.png" >/dev/null
sips -z 64 64 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_128x128.png" >/dev/null
sips -z 256 256 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_256x256.png" >/dev/null
sips -z 512 512 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_512x512.png" >/dev/null
sips -z 1024 1024 "${SOURCE_ICON}" --out "${ICONSET_DIR}/icon_512x512@2x.png" >/dev/null

python3 - "${ICONSET_DIR}" "${OUTPUT_ICON}" <<'PY'
from pathlib import Path
import sys

iconset = Path(sys.argv[1])
output = Path(sys.argv[2])
entries = [
    ("icp4", "icon_16x16.png"),
    ("icp5", "icon_32x32.png"),
    ("icp6", "icon_32x32@2x.png"),
    ("ic07", "icon_128x128.png"),
    ("ic08", "icon_256x256.png"),
    ("ic09", "icon_512x512.png"),
    ("ic10", "icon_512x512@2x.png"),
]

chunks = []
for icon_type, filename in entries:
    data = (iconset / filename).read_bytes()
    chunks.append(icon_type.encode("ascii") + (len(data) + 8).to_bytes(4, "big") + data)

body = b"".join(chunks)
output.write_bytes(b"icns" + (len(body) + 8).to_bytes(4, "big") + body)
PY

rm -rf "${ICONSET_DIR}"
