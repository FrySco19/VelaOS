#!/bin/sh
# Build the "Vela" and "Vela-Dark" icon themes from the packaged Papirus theme:
# Papirus everywhere, folders recolored to the Vela marine palette, Vela app icons on top.
# Idempotent; run as root at image build time (and after papirus-icon-theme upgrades).
set -e
SRC=/usr/share/icons/Papirus
DST=/usr/share/icons/Vela
DSTD=/usr/share/icons/Vela-Dark
[ -d "$SRC" ] || { echo "vela icons: Papirus not installed" >&2; exit 0; }
rm -rf "$DST" "$DSTD"
mkdir -p "$DST" "$DSTD"
# Papirus blue folder palette -> Vela
#   #5294e2 front   -> #3E8C9B   (sea)
#   #4877b1 back    -> #2B6A79   (deep sea)
#   #1d344f glyph   -> #0F3343   (marine)
#   #e4e4e4 paper   -> #EEF6F6   (foam)
recolor() { sed -e 's/#5294e2/#3E8C9B/gI' -e 's/#4877b1/#2B6A79/gI' -e 's/#1d344f/#0F3343/gI' -e 's/#e4e4e4/#EEF6F6/gI' "$1"; }
dirs=""
for d in "$SRC"/*/places; do
  [ -d "$d" ] || continue
  size=$(basename "$(dirname "$d")")
  out="$DST/$size/places"; mkdir -p "$out"
  for f in "$d"/*.svg; do
    n=$(basename "$f")
    if [ -L "$f" ]; then
      t=$(readlink "$f")
      case "$t" in
        *-blue*) ln -sf "$(echo "$t" | sed 's/-blue/-vela/')" "$out/$n" ;;
      esac
    else
      case "$n" in
        folder-blue*|user-blue*) recolor "$f" > "$out/$(echo "$n" | sed 's/-blue/-vela/')" ;;
        *) if grep -qi '#5294e2' "$f"; then recolor "$f" > "$out/$n"; fi ;;
      esac
    fi
  done
  # drop dangling links (targets that were not blue files)
  find "$out" -xtype l -delete 2>/dev/null || true
  dirs="$dirs${dirs:+,}$size/places"
  mkdir -p "$DSTD/$size"; ln -sfn "../../Vela/$size/places" "$DSTD/$size/places"
done
# index.theme: copy the per-directory sections for places from Papirus
sections() {
  awk -v want="$1" 'BEGIN{RS="";FS="\n"} { if ($1 ~ /^\[/) { h=$1; gsub(/[\[\]]/,"",h); n=split(want,a,","); for(i=1;i<=n;i++) if (a[i]==h) {print $0"\n"} } }' "$SRC/index.theme"
}
plain=$(echo "$dirs" | tr ',' '\n' | grep -v '@' | paste -sd, -)
scaled=$(echo "$dirs" | tr ',' '\n' | grep '@' | paste -sd, -)
for t in Vela Vela-Dark; do
  if [ "$t" = Vela ]; then inh="Papirus,breeze,hicolor"; nm="Vela"; else inh="Papirus-Dark,breeze-dark,hicolor"; nm="Vela scuro"; fi
  {
    echo "[Icon Theme]"
    echo "Name=$nm"
    echo "Comment=Vela (Papirus, cartelle Vela)"
    echo "Inherits=$inh"
    echo "Example=folder"
    echo "FollowsColorScheme=true"
    echo "DesktopDefault=48"
    echo "PanelDefault=48"
    echo "Directories=$plain"
    [ -n "$scaled" ] && echo "ScaledDirectories=$scaled"
    echo
    sections "$dirs"
  } > "/usr/share/icons/$t/index.theme"
done
command -v gtk-update-icon-cache >/dev/null && { gtk-update-icon-cache -qf "$DST" || true; gtk-update-icon-cache -qf "$DSTD" || true; }
echo "vela icons: $(find "$DST" -name '*.svg' | wc -l) entries"
