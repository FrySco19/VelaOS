#!/bin/sh
# Box-side driver: launch every default app and open every sample file type in the RUNNING dev VM,
# screenshot each, record which process handled it. Usage: components/apps/tests/devvm_apps.sh [OUTDIR]
# Needs: tools/devvm.sh start. Holds coordination/devvm.lock.
set -u
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
OUT=${1:-/workspace/vela/artifacts/apps/vm/$(date +%Y%m%d-%H%M)}
mkdir -p "$OUT"
VM="$ROOT/tools/vm.sh -n dev"
exec 8>"$ROOT/coordination/devvm.lock"; flock -w 1800 8 || { echo "devvm lock busy"; exit 1; }
ENV='export XDG_RUNTIME_DIR=/run/user/$(id -u) WAYLAND_DISPLAY=wayland-0 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus QT_QPA_PLATFORM=wayland LANG=it_IT.UTF-8;'
g() { $VM ssh "$ENV $*"; }

# sample files, generated in the guest
g 'set -e; d=/tmp/vela-samples; rm -rf $d; mkdir -p $d; cd $d
printf "Testo di prova di Vela.\nSeconda riga.\n" > nota.txt
printf "print(\"ciao\")\n" > script.py
ffmpeg -loglevel error -f lavfi -i testsrc=size=640x360:rate=25 -t 3 -pix_fmt yuv420p video.mp4
ffmpeg -loglevel error -f lavfi -i sine=frequency=440:duration=3 musica.mp3
ffmpeg -loglevel error -f lavfi -i testsrc=size=800x500 -frames:v 1 foto.png
ffmpeg -loglevel error -f lavfi -i testsrc=size=800x500 -frames:v 1 foto.jpg
cp nota.txt documento.txt && soffice --headless --convert-to odt documento.txt >/dev/null 2>&1 && soffice --headless --convert-to docx documento.txt >/dev/null 2>&1 && soffice --headless --convert-to pdf documento.txt >/dev/null 2>&1
printf "a;b\n1;2\n" > tabella.csv && soffice --headless --convert-to xlsx tabella.csv >/dev/null 2>&1
python3 -c "import zipfile; z=zipfile.ZipFile(\"archivio.zip\",\"w\"); z.write(\"nota.txt\"); z.close()"
ls -la' > "$OUT/samples.txt" 2>&1

: > "$OUT/results.tsv"
WAIT=${WAIT:-30}   # TCG (no KVM on the box): apps need tens of seconds to map a window
run() {  # name, command in guest, expected process regex, wait seconds
  name=$1; cmd=$2; expect=$3; wait=${4:-$WAIT}
  g "for p in \$(pgrep -u vela -f '$expect'); do grep -qa -e pgrep -e sshd /proc/\$p/cmdline 2>/dev/null || kill \$p 2>/dev/null; done; true"
  g "setsid sh -c '$cmd' >/tmp/vela-run-$name.log 2>&1 < /dev/null &" ; sleep "$wait"
  $VM screenshot "$OUT/$name.png" >/dev/null 2>&1
  proc=$(g "pgrep -u vela -a -f '$expect' | grep -v -e pgrep -e 'sh -c' -e kioclient | head -1")
  pid=$(echo "$proc" | awk '{print $1}')
  lang=""; [ -n "$pid" ] && lang=$(g "tr '\\0' '\\n' < /proc/$pid/environ | grep -E '^(LANG|LANGUAGE)=' | tr '\\n' ' '")
  st=FAIL; [ -n "$pid" ] && st=PASS
  printf '%s\t%s\t%s\t%s\n' "$st" "$name" "$(echo "$proc" | cut -c1-90)" "$lang" | tee -a "$OUT/results.tsv"
  [ -n "$pid" ] && g "kill $pid; sleep 1; kill -9 $pid 2>/dev/null; true"
  sleep 1
}
S=/tmp/vela-samples
# apps by desktop id (launched the way the menu does)
for a in firefox-esr:firefox org.kde.okular:okular org.kde.gwenview:gwenview org.kde.haruna:haruna org.kde.elisa:elisa \
         org.kde.kcalc:kcalc org.kde.kclock:kclock org.kde.kweather:kweather org.kde.marknote:marknote org.kde.ark:ark \
         org.kde.kwrite:kwrite org.kde.kate:'kate( |$)' org.kde.spectacle:spectacle org.kde.partitionmanager:partitionmanager \
         org.kde.plasma-systemmonitor:plasma-systemmonitor org.kde.filelight:filelight org.kde.skanpage:skanpage \
         org.kde.kdeconnect.app:kdeconnect-app libreoffice-writer:soffice libreoffice-calc:soffice libreoffice-impress:soffice \
         org.vela.spazi:vela_spazi btrfs-assistant:'kdialog|btrfs-assistant'; do
  id=${a%%:*}; re=${a#*:}
  run "app-$id" "kioclient exec /usr/share/applications/$id.desktop 2>/dev/null || kioclient exec /usr/local/share/applications/$id.desktop" "$re" "$WAIT"
done
# file types (KDE associations, as a double-click in File does)
for f in nota.txt:kwrite script.py:kate documento.pdf:okular foto.png:gwenview foto.jpg:gwenview video.mp4:haruna \
         musica.mp3:elisa documento.odt:soffice documento.docx:soffice tabella.xlsx:soffice archivio.zip:ark; do
  file=${f%%:*}; re=${f#*:}
  run "open-$file" "kioclient exec $S/$file" "$re" "$WAIT"
done
echo "results: $OUT/results.tsv"
