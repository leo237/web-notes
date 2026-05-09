#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Web Notes"
BIN_NAME="web-notes"
MIN_MACOS_VERSION="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
TARGETS=("aarch64-apple-darwin" "x86_64-apple-darwin")
DIST_DIR="target/macos-release"
PLIST_TEMPLATE="packaging/macos/Info.plist"
ICON_FILE="packaging/macos/WebNotes.icns"

export MACOSX_DEPLOYMENT_TARGET="${MIN_MACOS_VERSION}"

scripts/build-macos-icon.sh icon.png "${ICON_FILE}"
mkdir -p "${DIST_DIR}"

for target in "${TARGETS[@]}"; do
  rustup target add "${target}" >/dev/null
  cargo build --release --target "${target}"

  app_dir="${DIST_DIR}/${APP_NAME}-${target}.app"
  contents_dir="${app_dir}/Contents"
  macos_dir="${contents_dir}/MacOS"
  resources_dir="${contents_dir}/Resources"

  rm -rf "${app_dir}"
  mkdir -p "${macos_dir}" "${resources_dir}"
  cp "${PLIST_TEMPLATE}" "${contents_dir}/Info.plist"
  cp "${ICON_FILE}" "${resources_dir}/WebNotes.icns"
  cp "target/${target}/release/${BIN_NAME}" "${macos_dir}/${BIN_NAME}"
  chmod 755 "${macos_dir}/${BIN_NAME}"
done

if command -v lipo >/dev/null; then
  universal_app="${DIST_DIR}/${APP_NAME}-universal.app"
  rm -rf "${universal_app}"
  cp -R "${DIST_DIR}/${APP_NAME}-aarch64-apple-darwin.app" "${universal_app}"
  lipo -create \
    "${DIST_DIR}/${APP_NAME}-aarch64-apple-darwin.app/Contents/MacOS/${BIN_NAME}" \
    "${DIST_DIR}/${APP_NAME}-x86_64-apple-darwin.app/Contents/MacOS/${BIN_NAME}" \
    -output "${universal_app}/Contents/MacOS/${BIN_NAME}"
  chmod 755 "${universal_app}/Contents/MacOS/${BIN_NAME}"
  echo "Built ${universal_app}"
else
  echo "lipo not found; built architecture-specific app bundles in ${DIST_DIR}"
fi
