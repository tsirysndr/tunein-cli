#!/usr/bin/env bash
set -euo pipefail
# Reject path/sed metacharacters before using a version in filenames and metadata.
[[ "$VERSION" =~ ^v?[0-9][a-zA-Z0-9.+~-]*$ ]] || { echo "Invalid package version" >&2; exit 1; }
version_no_v="${VERSION#v}"
case "$TARGET" in
  x86_64-unknown-linux-gnu) deb_arch=amd64 ;;
  aarch64-unknown-linux-gnu) deb_arch=arm64 ;;
  armv7-unknown-linux-gnueabihf) deb_arch='' ;;
  *) echo "Unsupported target" >&2; exit 1 ;;
esac

name="tunein_${VERSION}_${TARGET}"
mkdir -p dist/staging
install -m 0755 target/tunein dist/staging/tunein
cp README.md LICENSE dist/staging/
tar -C dist/staging -czf "dist/${name}.tar.gz" .
(cd dist && shasum -a 256 "${name}.tar.gz" > "${name}.tar.gz.sha256")

# TuneIn armv7 has no Debian packaging metadata and ships a tarball only.
if [[ -n "$deb_arch" ]]; then
  pkg_root="dist/debian/${deb_arch}"
  mkdir -p "$pkg_root/usr/bin"
  install -m 0755 target/tunein "$pkg_root/usr/bin/tunein"
  sed -i "s/^Version: .*/Version: ${version_no_v}/" "$pkg_root/DEBIAN/control"
  dpkg-deb --build --root-owner-group "$pkg_root" "dist/tunein-cli_${version_no_v}_${deb_arch}.deb"
fi

# The existing release matrix ships RPMs only on x86_64.
if [[ "$TARGET" == x86_64-unknown-linux-gnu ]]; then
  src_root=dist/rpm/amd64
  mkdir -p "${src_root}/usr/bin"
  install -m 0755 target/tunein "${src_root}/usr/bin/tunein"
  sed -i "s/^Version:        .*/Version:        ${version_no_v}/" "${src_root}/tunein.spec"
  rpm_top="$(pwd)/dist/rpmbuild"
  mkdir -p "${rpm_top}"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  cp -r "${src_root}" "${rpm_top}/SOURCES/amd64"
  rpmbuild --define "_topdir ${rpm_top}" \
    --define "_sourcedir ${rpm_top}/SOURCES" \
    --target x86_64 -bb "${src_root}/tunein.spec"
  find "${rpm_top}/RPMS" -name '*.rpm' -exec mv {} dist/ \;
fi

cd dist
shopt -s nullglob
for package in *.deb *.rpm; do
  shasum -a 256 "$package" > "$package.sha256"
done
mkdir -p /out
cp ./*.tar.gz* ./*.deb* ./*.rpm* /out/
