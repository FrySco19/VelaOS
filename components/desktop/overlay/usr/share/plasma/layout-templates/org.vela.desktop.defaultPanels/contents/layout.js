// Vela OS 2 default layout (owned by component "desktop").
// Top bar (macOS-like menu bar) + floating centered dock + desktop folder view.
// Loaded from the look-and-feel layout via loadTemplate("org.vela.desktop.defaultPanels").

// ---------- Desktop (folder view) ----------
var desktops = desktopsForActivity(currentActivity());
for (var i = 0; i < desktops.length; i++) {
    var d = desktops[i];
    // Right-click menu entries (Apri terminale, Impostazioni schermo, ...) are set in the
    // corona-wide [ActionPlugins] group seeded by /etc/skel/.config/plasma-org.kde.plasma.desktop-appletsrc:
    // the context-menu plugin reads it before this script runs, so it must exist at first login.
    // Wallpaper package from component "theme" (switches light/dark by itself).
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    d.writeConfig("Image", "file:///usr/share/wallpapers/Vela/");
    d.currentConfigGroup = ["General"];
    d.writeConfig("arrangement", 1);       // 0 = rows, 1 = columns
    d.writeConfig("alignment", 0);         // left
    d.writeConfig("sortMode", -1);         // keep user positions (manual)
    d.writeConfig("iconSize", 3);          // large-ish desktop icons
    d.writeConfig("popups", true);         // spring-loaded folders on drag
    d.writeConfig("previews", true);
    d.writeConfig("selectionMarkers", false);
    d.currentConfigGroup = [];
}

// ---------- Top bar ----------
var top = new Panel;
top.location = "top";
top.height = 2 * Math.round(gridUnit * 1.6 / 2); // ~28-30 px
top.floating = false;
top.hiding = "none";
top.lengthMode = "fill";

var launcher = top.addWidget("org.kde.plasma.kickoff");
launcher.currentConfigGroup = ["General"];
launcher.writeConfig("icon", "vela-logo-symbolic");   // shipped by component theme
launcher.writeConfig("favoritesDisplay", 0);          // grid
launcher.writeConfig("applicationsDisplay", 0);       // grid like Launchpad
launcher.writeConfig("switchCategoryOnHover", true);
launcher.writeConfig("alphaSort", true);
launcher.writeConfig("showActionButtonCaptions", true);
launcher.currentConfigGroup = ["Shortcuts"];
launcher.writeConfig("global", "Alt+F1");

// Name of the active application (like the bold app name on macOS); click = window list.
// (org.vela.appname = Window List fork: bold name, "Scrivania" when the desktop is focused)
top.addWidget("org.vela.appname");

// Global menu of the active application.
var menu = top.addWidget("org.kde.plasma.appmenu");
menu.currentConfigGroup = ["Appearance"];
menu.writeConfig("compactView", false);

top.addWidget("org.kde.plasma.panelspacer");

top.addWidget("org.kde.plasma.systemtray");

var clock = top.addWidget("org.kde.plasma.digitalclock");
clock.currentConfigGroup = ["Appearance"];
clock.writeConfig("showDate", true);
clock.writeConfig("dateDisplayFormat", "BesideTime");
clock.writeConfig("dateFormat", "custom");
clock.writeConfig("customDateFormat", "ddd d MMM");
clock.writeConfig("use24hFormat", 1);       // follow locale (24h in Italian)
clock.writeConfig("showSeconds", 0);

// ---------- Dock ----------
var dock = new Panel;
dock.location = "bottom";
dock.height = 2 * Math.round(gridUnit * 3 / 2);   // ~54 px
dock.floating = true;
dock.alignment = "center";
dock.lengthMode = "fit";
dock.hiding = "none";        // always visible: easier for new users

var tasks = dock.addWidget("org.kde.plasma.icontasks");

function firstExisting(list) {
    for (var k = 0; k < list.length; k++) {
        if (applicationExists(list[k])) {
            return list[k];
        }
    }
    return "";
}

// Pinned apps, in order; ids that are not installed are skipped.
var pinned = ["preferred://filemanager", "preferred://browser"];
var candidates = [
    ["org.vela.assistant.desktop", "vela-assistant.desktop"],
    ["vela-ai-terminal.desktop", "org.vela.terminal-ai.desktop", "org.vela.terminal.desktop", "vela-terminal-ai.desktop"],
    ["org.kde.konsole.desktop"],
    ["org.kde.discover.desktop"],
    ["systemsettings.desktop"]
];
for (var c = 0; c < candidates.length; c++) {
    var id = firstExisting(candidates[c]);
    if (id !== "" && pinned.indexOf("applications:" + id) < 0) {
        pinned.push("applications:" + id);
    }
}
tasks.currentConfigGroup = ["General"];
tasks.writeConfig("launchers", pinned);   // must be a JS array (a comma string is dropped)
tasks.writeConfig("showOnlyCurrentDesktop", false);
tasks.writeConfig("showOnlyCurrentActivity", true);
tasks.writeConfig("groupingStrategy", 1);        // group windows of the same app
tasks.writeConfig("highlightWindows", true);
tasks.writeConfig("indicateAudioStreams", true);
tasks.writeConfig("middleClickAction", 2);       // new instance
tasks.writeConfig("iconSpacing", 1);

dock.addWidget("org.kde.plasma.marginsseparator");
dock.addWidget("org.kde.plasma.trash");
