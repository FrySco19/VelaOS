"use strict";
// Context menus + toolbar button. Pending actions are queued until the sidebar asks for them.
const _ = (k, s) => browser.i18n.getMessage(k, s);
let pending = null;

browser.runtime.onInstalled.addListener(() => {
  browser.contextMenus.removeAll().then(() => {
    browser.contextMenus.create({ id: "vela-summarize", title: _("menuSummarize"), contexts: ["page"] });
    browser.contextMenus.create({ id: "vela-ask-sel", title: _("menuAskSel"), contexts: ["selection"] });
  });
});

browser.contextMenus.onClicked.addListener((info, tab) => {
  // sidebarAction.open() must run synchronously inside the user-action handler.
  browser.sidebarAction.open();
  pending = info.menuItemId === "vela-summarize"
    ? { action: "summarize", tabId: tab.id }
    : { action: "askSelection", tabId: tab.id, selection: info.selectionText || "" };
  browser.runtime.sendMessage({ type: "pending" }).catch(() => {});
});

browser.browserAction.onClicked.addListener(() => { browser.sidebarAction.toggle(); });

browser.runtime.onMessage.addListener((msg) => {
  if (msg && msg.type === "takePending") { const p = pending; pending = null; return Promise.resolve(p); }
  return false;
});
