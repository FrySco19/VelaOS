// Vela defaults for Firefox ESR. The user can change these preferences in Firefox settings.
// Debian loads every *.js file in /etc/firefox-esr as default preferences.

// Native KDE dialogs (file picker, "open with") through xdg-desktop-portal-kde.
pref("widget.use-xdg-desktop-portal.file-picker", 1);
pref("widget.use-xdg-desktop-portal.mime-handler", 1);
pref("widget.use-xdg-desktop-portal.open-uri", 1);

// Quiet first run: no welcome tour, no "what's new" page, no default-browser nag, no data-submission bar.
pref("browser.aboutwelcome.enabled", false);
pref("browser.startup.homepage_override.mstone", "ignore");
pref("startup.homepage_welcome_url", "");
pref("startup.homepage_welcome_url.additional", "");
pref("datareporting.policy.dataSubmissionPolicyBypassNotification", true);
pref("browser.shell.checkDefaultBrowser", false);

// Clean new tab: search + your frequent sites only. No sponsored tiles, stories, weather or Pocket.
pref("browser.newtabpage.activity-stream.showSponsored", false);
pref("browser.newtabpage.activity-stream.showSponsoredTopSites", false);
pref("browser.newtabpage.activity-stream.feeds.section.topstories", false);
pref("browser.newtabpage.activity-stream.feeds.topsites", true);
pref("browser.newtabpage.activity-stream.showWeather", false);
pref("browser.newtabpage.activity-stream.feeds.weatherfeed", false);
pref("browser.newtabpage.activity-stream.section.highlights.includePocket", false);
pref("browser.newtabpage.activity-stream.showSearch", true);
pref("browser.newtabpage.activity-stream.feeds.snippets", false);
pref("browser.urlbar.suggest.quicksuggest.sponsored", false);
pref("browser.urlbar.suggest.quicksuggest.nonsponsored", false);

// Privacy: strict tracking protection, HTTPS-only mode, Global Privacy Control, no telemetry.
pref("browser.contentblocking.category", "strict");
pref("dom.security.https_only_mode", true);
pref("privacy.globalprivacycontrol.enabled", true);
pref("toolkit.telemetry.enabled", false);
pref("toolkit.telemetry.unified", false);
pref("datareporting.healthreport.uploadEnabled", false);
pref("app.shield.optoutstudies.enabled", false);

// One AI sidebar: Vela's local-first assistant extension. Firefox's built-in chatbot only offers online providers.
pref("browser.ml.chat.enabled", false);
pref("browser.ml.chat.shortcuts", false);
// New sidebar with the extension icons on the side (Firefox 153 default; set explicitly).
pref("sidebar.revamp", true);

// Smooth scrolling and pinch zoom feel closer to macOS.
pref("general.smoothScroll", true);
pref("general.smoothScroll.msdPhysics.enabled", true);
pref("apz.gtk.kinetic_scroll.enabled", true);

// Downloads go to ~/Download without asking each time (like Safari/Edge).
pref("browser.download.useDownloadDir", true);
pref("browser.download.always_ask_before_handling_new_types", false);
