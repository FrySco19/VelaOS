#!/bin/sh
# assistant: byte-compile, register the global shortcut (Meta+A) with kglobalaccel, permissions.
set -e
A=/usr/lib/vela/assistant
chmod 0755 /usr/bin/vela-assistant "$A/vela_assistant.py" "$A/krunner_runner.py"
python3 -m compileall -q "$A" >/dev/null 2>&1 || true
# kglobalaccel reads default shortcuts (X-KDE-Shortcuts) from desktop files in /usr/share/kglobalaccel
mkdir -p /usr/share/kglobalaccel
ln -sf /usr/share/applications/org.vela.assistant.desktop /usr/share/kglobalaccel/org.vela.assistant.desktop
exit 0
