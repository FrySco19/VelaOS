#!/bin/bash
# System-level checks over SSH (dev VM or live ISO with SSH). IDs: P-IT-1 R1 R1b P-NET-1 P-AUDIO-1 R6 P-A11Y-1 V-2 P-BOOT
. "$(dirname "$0")/../lib/qa.sh"
gwait_ssh 120 || { record FAIL ssh "guest SSH unreachable on port $QA_SSH_PORT"; exit 1; }
gssh 'set +e
echo "== os"; cat /etc/os-release | head -4
echo "== locale"; localectl status; locale | head -3
echo "== tz"; timedatectl show -p Timezone --value
echo "== session"; loginctl list-sessions --no-legend; for s in $(loginctl list-sessions --no-legend | awk "{print \$1}"); do loginctl show-session $s -p Type -p Desktop -p Name; done
echo "== plasma"; plasmashell --version 2>/dev/null; kwin_wayland --version 2>/dev/null
echo "== mem"; free -m
echo "== boot"; systemd-analyze 2>/dev/null; systemd-analyze blame 2>/dev/null | head -8
echo "== failed"; systemctl --failed --no-legend; systemctl --user --failed --no-legend 2>/dev/null
echo "== modules"; lsmod | grep -E "vbox|vmwgfx|virtio_gpu|qxl|bochs" 
echo "== net"; nmcli -t -f STATE general 2>/dev/null; nmcli -t -f DEVICE,STATE dev 2>/dev/null
echo "== audio"; pactl info 2>/dev/null | grep -E "Server Name|Default Sink"; wpctl status 2>/dev/null | sed -n "1,25p"
echo "== flatpak"; flatpak remotes 2>/dev/null
echo "== fonts"; fc-match sans-serif; fc-match monospace
echo "== pkgs"; for p in plasma-discover plasma-discover-backend-flatpak calamares orca systemsettings dolphin konsole kde-spectacle xdg-user-dirs; do printf "%s=%s\n" $p "$(dpkg-query -W -f="\${Status}" $p 2>/dev/null | grep -q "install ok installed" && echo yes || echo no)"; done
echo "== userdirs"; cat ~/.config/user-dirs.dirs 2>/dev/null | grep -v "^#"; ls ~
' > "$QA_OUT/system.log" 2>&1
L="$QA_OUT/system.log"
grep -q 'LANG=it_IT' "$L" && record PASS P-IT-1 "locale it_IT" "$L" || record FAIL P-IT-1 "system locale is not it_IT" "$L"
grep -qiE 'X11 Layout: it|VC Keymap: it' "$L" && record PASS P-IT-1 "keyboard it" "$L" || record FAIL P-IT-1 "keyboard layout not it" "$L"
grep -q 'Europe/Rome' "$L" && record PASS P-IT-1 "timezone Europe/Rome" "$L" || record FAIL P-IT-1 "timezone not Europe/Rome" "$L"
grep -q 'Type=wayland' "$L" && record PASS R1 "Plasma session on Wayland" "$L" || record WARN R1 "graphical session not Wayland (or none)" "$L"
grep -qE 'Desktop=KDE' "$L" && record PASS P-DESK-1 "KDE session running" "$L" || record FAIL P-DESK-1 "no KDE graphical session found" "$L"
used=$(awk '/^Mem:/{print $3; exit}' "$L"); [ -n "$used" ] && { [ "$used" -le 1536 ] && record PASS R1 "idle RAM used ${used} MiB (≤1536)" "$L" || record FAIL R1 "idle RAM used ${used} MiB (>1536)" "$L"; }
fl=$(sed -n '/== failed/,/== modules/p' "$L" | grep -vc '^==' ); [ "$fl" -eq 0 ] && record PASS P-BOOT-1 "no failed systemd units" "$L" || record FAIL P-BOOT-1 "$fl failed systemd unit lines" "$L"
grep -qE 'STATE.*connected|^connected' "$L" && record PASS P-NET-1 "NetworkManager connected" "$L" || record FAIL P-NET-1 "NetworkManager not connected" "$L"
grep -q 'PipeWire' "$L" && record PASS P-AUDIO-1 "PipeWire server" "$L" || record FAIL P-AUDIO-1 "PipeWire not running in session" "$L"
grep -q '^flathub' "$L" && record PASS R6 "flathub remote configured" "$L" || record FAIL R6 "no flathub remote" "$L"
for p in plasma-discover plasma-discover-backend-flatpak systemsettings dolphin konsole orca kde-spectacle; do grep -q "^$p=yes" "$L" && record PASS pkg "$p installed" || record FAIL pkg "$p missing" "$L"; done
sed -n '/== fonts/,/== pkgs/p' "$L" | grep -q 'Inter' && record PASS V-2 "sans-serif resolves to Inter" "$L" || record FAIL V-2 "sans-serif is not Inter" "$L"
grep -qE 'Documenti|Scrivania' "$L" && record PASS P-FM-1 "Italian XDG user dirs" "$L" || record FAIL P-FM-1 "XDG user dirs not Italian" "$L"
