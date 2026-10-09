#!/bin/sh
# Vela OS 2 — component "desktop" (behaviour, layout, shortcuts, Dolphin, desktop icons).
# Runs as root inside the chroot. Idempotent. Only writes the KEYS desktop owns
# into shared /etc/xdg files (see coordination/requests.md).
set -e
# VELA_ROOT lets tests run the hook against a scratch tree instead of /.
ROOT=${VELA_ROOT:-}

KW=$(command -v kwriteconfig6 || true)
if [ -z "$KW" ]; then
    echo "desktop: kwriteconfig6 missing (package libkf6config-bin)" >&2
    exit 1
fi
# kw FILE GROUP[/SUBGROUP...] KEY VALUE [TYPE]
kw() {
    f=$1; g=$2; k=$3; v=$4; t=${5:-}
    set -- --file "$ROOT/etc/xdg/$f"
    old_ifs=$IFS; IFS='/'
    for part in $g; do set -- "$@" --group "$part"; done
    IFS=$old_ifs
    [ -n "$t" ] && set -- "$@" --type "$t"
    "$KW" "$@" --key "$k" "$v"
}
TAB=$(printf '\t')

# ---------------- KWin: window behaviour ----------------
kw kwinrc Windows Placement Centered
kw kwinrc Windows ElectricBorderMaximize true bool          # drag to top = maximize
kw kwinrc Windows ElectricBorderTiling true bool            # drag to side/corner = half/quarter tile
kw kwinrc Windows ElectricBorderCornerRatio 0.25
kw kwinrc Windows RollOverDesktops false bool
kw kwinrc Windows FocusStealingPreventionLevel 1
kw kwinrc Windows TitlebarDoubleClickCommand Maximize
# Hot corner top-left = Panoramica (overview); Meta alone = Panoramica (like Mission Control)
kw kwinrc Effect-overview BorderActivate 7
kw kwinrc ModifierOnlyShortcuts Meta "org.kde.kglobalaccel,/component/kwin,org.kde.kglobalaccel.Component,invokeShortcut,Overview"
kw kwinrc Plugins overviewEnabled true bool
kw kwinrc Plugins windowviewEnabled true bool
# Alt+Tab with live thumbnails
kw kwinrc TabBox LayoutName thumbnail_grid
kw kwinrc TabBox HighlightWindows true bool
kw kwinrc TabBox ShowTabBox true bool
kw kwinrc TabBoxAlternative LayoutName thumbnail_grid
# Spazi = virtual desktops (apps' "Spazi" app adds more at runtime)
kw kwinrc Desktops Number 2
kw kwinrc Desktops Rows 1
kw kwinrc Desktops Name_1 "Spazio 1"
kw kwinrc Desktops Name_2 "Spazio 2"

# ---------------- Global shortcuts ----------------
# Ricerca (KRunner) like Spotlight / Windows search
kw kglobalshortcutsrc services/org.kde.krunner.desktop _launch "Alt+Space${TAB}Meta+Space${TAB}Alt+F2${TAB}Search"
# Screenshot: Print = whole screen, Meta+Shift+S = area (like Windows)
kw kglobalshortcutsrc services/org.kde.spectacle.desktop _launch "Print"
kw kglobalshortcutsrc services/org.kde.spectacle.desktop RectangularRegionScreenShot "Meta+Shift+S${TAB}Meta+Shift+Print"
kw kglobalshortcutsrc services/vela-ai-terminal.desktop _launch "Meta+Shift+T"
kw krunnerrc General FreeFloating true bool                # centred, Spotlight-like
kw krunnerrc General ActivateWhenTypingOnDesktop true bool
kw krunnerrc General historyBehavior CompletionSuggestion
kw krunnerrc Plugins baloosearchEnabled true bool
kw krunnerrc Plugins calculatorEnabled true bool
kw krunnerrc Plugins unitconverterEnabled true bool
kw krunnerrc Plugins krunner_systemsettingsEnabled true bool
kw krunnerrc Plugins krunner_webshortcutsEnabled false bool # no surprise web jumps

# ---------------- Spectacle ----------------
kw spectaclerc General clipboardGroup PostScreenshotCopyImage
kw spectaclerc General autoSaveImage true bool
kw spectaclerc General launchAction TakeFullscreenScreenshot

# ---------------- Clipboard (Klipper, Meta+V history) ----------------
kw klipperrc General KeepClipboardContents true bool
kw klipperrc General MaxClipItems 30
kw klipperrc General IgnoreImages false bool
kw klipperrc General SyncClipboards false bool
kw klipperrc General PreventEmptyClipboard true bool

# ---------------- General behaviour ----------------
kw kdeglobals KDE SingleClick false bool                 # double-click opens (macOS/Windows)
kw kdeglobals General BrowserApplication firefox-esr.desktop
kw kdeglobals PreviewSettings MaximumRemoteSize 5242880
kw kdeglobals KFileDialog\ Settings "Show hidden files" false bool

# ---------------- Dolphin = "File", configured like Finder ----------------
kw dolphinrc General ShowFullPath false bool             # breadcrumb
kw dolphinrc General EditableUrl false bool
kw dolphinrc General RememberOpenedTabs false bool
kw dolphinrc General OpenExternallyCalledFolderInNewTab false bool
kw dolphinrc General BrowseThroughArchives false bool
kw dolphinrc General ShowSelectionToggle true bool
kw dolphinrc General ConfirmClosingMultipleTabs true bool
kw dolphinrc General ShowStatusBar true bool
kw dolphinrc General ShowZoomSlider true bool
kw dolphinrc General ShowSpaceInfo true bool
kw dolphinrc General GlobalViewProps true bool
kw dolphinrc General UseTabForSwitchingSplitView true bool
kw dolphinrc General CloseActiveSplitView true bool
kw dolphinrc General ShowCopyMoveMenu true bool
kw dolphinrc MainWindow MenuBar Enabled                   # menus go to the top bar (global menu)
kw dolphinrc ContentDisplay DirectorySizeMode ContentCount
kw dolphinrc ContextMenu ShowOpenInNewTab true bool
kw dolphinrc ContextMenu ShowOpenInSplitView true bool
kw dolphinrc ContextMenu ShowCopyLocation true bool
kw dolphinrc ContextMenu ShowDuplicateHere true bool
kw dolphinrc ContextMenu ShowSortBy true bool
kw dolphinrc ContextMenu ShowViewMode true bool
kw dolphinrc PreviewSettings Plugins "appimagethumbnail,audiothumbnail,comicbookthumbnail,cursorthumbnail,directorythumbnail,djvuthumbnail,ebookthumbnail,exrthumbnail,fontthumbnail,imagethumbnail,jpegthumbnail,kraorathumbnail,opendocumentthumbnail,svgthumbnail,windowsexethumbnail,windowsimagethumbnail,ffmpegthumbs,gsthumbnail,rawthumbnail,mobithumbnail,blenderthumbnail"
kw dolphinrc InformationPanel previewsAutoPlay true bool
kw dolphinrc InformationPanel showHovered false bool
kw dolphinrc IconsMode PreviewSize 64
kw dolphinrc IconsMode IconSize 64

# ---------------- Desktop integration (Vela) ----------------
chmod 0755 "$ROOT/usr/lib/vela/desktop/vela-desktop-sync" "$ROOT/usr/bin/vela-desktop-settings"
if command -v systemctl >/dev/null 2>&1; then
    systemctl --root="${ROOT:-/}" --global enable vela-desktop-session.service vela-desktop-apps.path vela-desktop-drives.path
fi

# Layout: theme's look-and-feel layout should be `loadTemplate("org.vela.desktop.defaultPanels")`.
# Fallback if it ships none: add it (never overwrite theme files); if no Vela
# look-and-feel is configured at all, divert Breeze's layout to ours.
LNF=$(kreadconfig6 --file "$ROOT/etc/xdg/kdeglobals" --group KDE --key LookAndFeelPackage 2>/dev/null || true)
[ -n "$LNF" ] || LNF=org.kde.breeze.desktop
L="$ROOT/usr/share/plasma/look-and-feel/$LNF/contents/layouts/org.kde.plasma.desktop-layout.js"
if [ -d "$ROOT/usr/share/plasma/look-and-feel/$LNF" ] && ! grep -qs org.vela.desktop.defaultPanels "$L"; then
    echo "desktop: look-and-feel $LNF has no Vela layout; installing loadTemplate fallback" >&2
    if [ -f "$L" ] && [ -z "$ROOT" ] && dpkg -S "$L" >/dev/null 2>&1; then
        dpkg-divert --quiet --local --rename --add "$L"
    fi
    if [ ! -f "$L" ]; then
        mkdir -p "$(dirname "$L")"
        printf 'loadTemplate("org.vela.desktop.defaultPanels")\n' > "$L"
    fi
fi

# Make xdg-user-dirs create the localized folders (Scrivania, Documenti...) at first login
if [ -f "$ROOT/etc/xdg/user-dirs.conf" ]; then
    sed -i 's/^enabled=.*/enabled=True/' "$ROOT/etc/xdg/user-dirs.conf"
fi
echo "desktop: hook done"
