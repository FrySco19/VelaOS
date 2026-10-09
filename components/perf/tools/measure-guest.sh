#!/bin/sh
# Vela perf — measurements INSIDE the guest (run as the desktop user, e.g. via tools/vm.sh ssh).
#   sh measure-guest.sh [--apps] [--idle SECS] [--fps]
# Prints a plain report; every number comes from the running system (systemd-analyze, journal, /proc, KWin).
set -u
APPS=0; IDLE=90; FPS=0
while [ $# -gt 0 ]; do case $1 in --apps) APPS=1;; --idle) IDLE=$2; shift;; --fps) FPS=1;; esac; shift; done
U=$(id -u)
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$U}
export DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}
[ -n "${WAYLAND_DISPLAY:-}" ] || for s in "$XDG_RUNTIME_DIR"/wayland-[0-9]; do [ -S "$s" ] && export WAYLAND_DISPLAY=$(basename "$s"); done
export DISPLAY=${DISPLAY:-:0}
QDBUS=$(command -v qdbus6 || command -v qdbus || echo qdbus6)
sec() { awk -v a="$1" 'BEGIN{printf "%.2f", a/1000000}'; }

echo "== system"
echo "date: $(date -Is)"; echo "kernel: $(uname -r)"; echo "virt: $(systemd-detect-virt 2>/dev/null)"
echo "cpus: $(nproc)  mem_total_mb: $(awk '/MemTotal/{printf "%d",$2/1024}' /proc/meminfo)"
echo "live: $( [ -d /run/live/medium ] && echo yes || echo no)"
[ -r /run/vela/gpu ] && sed 's/^/gpu./' /run/vela/gpu

echo "== boot"
systemd-analyze 2>&1 | head -3
# desktop ready = Plasma's splash-ready unit (user manager), monotonic since kernel start
dr=$(journalctl --user -b -o short-monotonic --no-pager 2>/dev/null | grep -m1 -E "Finished plasma-ksplash-ready.service|Started plasma-ksplash-ready.service|Started plasma-plasmashell.service" | sed -E 's/^\[ *([0-9.]+)\].*/\1/')
echo "desktop_ready_s_since_kernel: ${dr:-n/a}"
echo "-- critical chain"; systemd-analyze critical-chain graphical.target 2>/dev/null | sed -n '1,25p'
echo "-- blame (top 15)"; systemd-analyze blame 2>/dev/null | head -15
echo "-- user blame (top 10)"; systemd-analyze --user blame 2>/dev/null | head -10

echo "== session"
echo "session_type: $(loginctl show-session "$(loginctl list-sessions --no-legend 2>/dev/null | awk -v u="$(id -un)" '$3==u{print $1; exit}')" -p Type --value 2>/dev/null)"
info=$("$QDBUS" org.kde.KWin /KWin org.kde.KWin.supportInformation 2>/dev/null)
printf '%s\n' "$info" | grep -E "^(OpenGL renderer string|OpenGL version string|Compositing Type|Platform|Animation speed|Operation Mode|OpenGL platform interface):" | sed 's/^/kwin./'
echo "kwin.loaded_effects: $("$QDBUS" org.kde.KWin /Effects org.freedesktop.DBus.Properties.Get org.kde.kwin.Effects loadedEffects 2>/dev/null | tr '\n' ' ')"
st="${XDG_STATE_HOME:-$HOME/.local/state}/vela"; [ -r "$st/graphics-mode" ] && echo "vela.graphics_mode: $(cat "$st/graphics-mode")"

echo "== memory"
up=$(cut -d. -f1 /proc/uptime)
if [ "$up" -lt "$IDLE" ]; then echo "(waiting until uptime ${IDLE}s for idle numbers)"; sleep $((IDLE - up)); fi
free -m
echo "idle_used_mb: $(free -m | awk '/^Mem:/{print $3}')  (uptime $(cut -d. -f1 /proc/uptime)s)"
zramctl 2>/dev/null; swapon --show 2>/dev/null
echo "-- top RSS"; ps -eo rss=,comm= --sort=-rss | head -12 | awk '{printf "%7.1f MB  %s\n",$1/1024,$2}'

if [ "$APPS" = 1 ]; then
  echo "== app launch (time from exec to first window, KWin windows runner)"
  win_count() { busctl --user call org.kde.KWin /WindowsRunner org.kde.krunner1 Match s "$1" 2>/dev/null | awk '{print $2+0}'; }
  for spec in "dolphin:dolphin" "konsole:konsole" "systemsettings:systemsettings" "plasma-discover:discover" "kate:kate" "firefox-esr:firefox"; do
    bin=${spec%%:*}; q=${spec#*:}
    command -v "$bin" >/dev/null 2>&1 || { echo "$bin: not installed"; continue; }
    for run in 1 2; do
      base=$(win_count "$q"); base=${base:-0}
      t0=$(date +%s%N); setsid "$bin" >/dev/null 2>&1 & pid=$!
      ok=0; while [ $(( ($(date +%s%N) - t0) / 1000000 )) -lt 20000 ]; do
        n=$(win_count "$q"); [ "${n:-0}" -gt "$base" ] && { ok=1; break; }; sleep 0.05; done
      ms=$(( ($(date +%s%N) - t0) / 1000000 ))
      [ $ok = 1 ] && echo "$bin run$run: ${ms} ms" || echo "$bin run$run: no window after 20 s"
      sleep 2; pkill -TERM -P $pid 2>/dev/null; kill -TERM $pid 2>/dev/null; pkill -TERM -x "$bin" 2>/dev/null; sleep 2
    done
  done
fi

if [ "$FPS" = 1 ] && command -v eglgears_wayland >/dev/null 2>&1 && [ -n "${WAYLAND_DISPLAY:-}" ]; then
  echo "== GL frame rate inside the session (eglgears_wayland, 12 s, 600x400 window)"
  timeout 12 eglgears_wayland 2>&1 | grep -i "frames" | tail -2
fi
