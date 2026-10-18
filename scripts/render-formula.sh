#!/bin/bash
# Write the Homebrew formulae for a release.
#
#   scripts/render-formula.sh <version> <checksums.txt> <output dir> [download base URL]
#
# Writes sam.rb (window only) and sam-tui.rb (terminal UI only).
#
# Homebrew downloads formula archives with curl, so Sam.app arrives without the
# quarantine flag and Gatekeeper does not block the ad-hoc signed app.

# macOS /bin/bash is 3.2. Keep this script compatible with that.

set -eu
set -o pipefail

if [[ $# -lt 3 ]]; then
  echo "usage: $0 <version> <checksums.txt> <output dir> [download base URL]" >&2
  exit 1
fi

version="${1#v}"
checksums="$2"
outdir="$3"
base="${4:-https://github.com/luisitoluis/sam-macos-rs/releases/download/v$version}"

sha_for() {
  local archive="sam_${version}_darwin_$1.tar.gz" sha
  sha="$(awk -v f="$archive" '$2 == f || $2 == "*" f { print $1 }' "$checksums")"
  [[ -n "$sha" ]] || { echo "$checksums has no entry for $archive" >&2; exit 1; }
  echo "$sha"
}

arm_sha="$(sha_for arm64)"
intel_sha="$(sha_for amd64)"

mkdir -p "$outdir"

urls() {
  cat <<EOF
  on_arm do
    url "$base/sam_${version}_darwin_arm64.tar.gz"
    sha256 "$arm_sha"
  end
  on_intel do
    url "$base/sam_${version}_darwin_amd64.tar.gz"
    sha256 "$intel_sha"
  end
EOF
}

cat >"$outdir/sam.rb" <<EOF
class Sam < Formula
  desc "Steam Achievement Manager for macOS"
  homepage "https://github.com/luisitoluis/sam-macos-rs"
  version "$version"
  license "GPL-3.0-only"

$(urls)

  depends_on :macos

  def install
    prefix.install "Sam.app"
    (bin/"sam").write <<~SH
      #!/bin/sh
      exec open -a "#{opt_prefix}/Sam.app" --args "\$@"
    SH
    (bin/"sam").chmod 0755
  end

  def caveats
    <<~EOS
      Run \`sam\` to open the window.
      The terminal UI is a separate formula: \`brew install luisitoluis/tap/sam-tui\`.
      Steam needs to have been launched once before the first run.
    EOS
  end

  test do
    assert_predicate prefix/"Sam.app/Contents/Info.plist", :exist?
  end
end
EOF

cat >"$outdir/sam-tui.rb" <<EOF
class SamTui < Formula
  desc "Steam Achievement Manager for macOS (terminal UI)"
  homepage "https://github.com/luisitoluis/sam-macos-rs"
  version "$version"
  license "GPL-3.0-only"

$(urls)

  depends_on :macos

  def install
    bin.install "sam-tui"
  end

  def caveats
    <<~EOS
      Run \`sam-tui\` for the terminal UI.
      The window is a separate formula: \`brew install luisitoluis/tap/sam\`.
      Steam needs to have been launched once before the first run.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sam-tui --version")
  end
end
EOF
