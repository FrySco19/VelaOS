"use strict";
const _ = (k, s) => browser.i18n.getMessage(k, s) || k;
const $ = (id) => document.getElementById(id);
const HOST = "org.vela.ai";
const RESTRICTED = /^(about:|moz-extension:|chrome:|resource:|view-source:|file:.*\.pdf$|https:\/\/(addons\.mozilla\.org|accounts-static\.cdn\.mozilla\.net))/i;

const state = { windowId: null, tabId: null, tabs: new Map(), port: null, busy: false, status: null, reqId: 0 };
const conv = (tabId) => { if (!state.tabs.has(tabId)) state.tabs.set(tabId, { messages: [], nodes: [] }); return state.tabs.get(tabId); };

// ---------- i18n ----------
for (const el of document.querySelectorAll("[data-i18n]")) el.textContent = _(el.dataset.i18n);
for (const el of document.querySelectorAll("[data-i18n-placeholder]")) el.placeholder = _(el.dataset.i18nPlaceholder);
for (const el of document.querySelectorAll("[data-i18n-title]")) { el.title = _(el.dataset.i18nTitle); el.setAttribute("aria-label", el.title); }
document.documentElement.lang = browser.i18n.getUILanguage().slice(0, 2);

// ---------- native messaging ----------
function nativeRequest(msg, onChunk) {
  return new Promise((resolve, reject) => {
    let port;
    try { port = browser.runtime.connectNative(HOST); } catch (e) { reject(new Error(_("noHost"))); return; }
    const id = ++state.reqId; let settled = false;
    const finish = (fn, v) => { if (settled) return; settled = true; if (state.port === port) state.port = null; try { port.disconnect(); } catch (e) {} fn(v); };
    port.onMessage.addListener((m) => {
      if (m.id !== id) return;
      if (m.type === "chunk") onChunk && onChunk(m.text);
      else if (m.type === "done") finish(resolve, m);
      else if (m.type === "error") finish(reject, new Error(m.message || _("errorGeneric")));
    });
    port.onDisconnect.addListener((p) => {
      if (settled) return;
      settled = true; if (state.port === port) state.port = null;
      const err = new Error(p.error ? _("noHost") : _("stopped")); err.stopped = !p.error; reject(err);
    });
    if (onChunk) state.port = port;
    port.postMessage({ id, ...msg });
  });
}
function stop() {
  if (state.port) { const p = state.port; state.port = null; try { p.disconnect(); } catch (e) {} }
}

// ---------- status ----------
async function refreshStatus() {
  const box = $("model"), txt = $("modelText"), banner = $("banner");
  try {
    const s = await nativeRequest({ cmd: "status" });
    state.status = s;
    box.className = "model " + (!s.ok ? "off" : s.cloud ? "cloud" : "ok");
    txt.textContent = s.ok ? `${s.cloud ? _("onlineService") : _("localModel")} · ${s.model || "—"}` : _("disabled");
    box.title = txt.textContent;
    if (!s.ok) showBanner(s.message, true);
    else if (s.cloud) showBanner(_("onlineWarn"), false);
    else banner.hidden = true;
  } catch (e) {
    state.status = { ok: false };
    box.className = "model off"; txt.textContent = _("disabled"); showBanner(e.message, false);
  }
  updateButtons();
}
function showBanner(text, withSettings) {
  const b = $("banner"); b.textContent = text; b.hidden = false;
  if (withSettings) {
    const btn = document.createElement("button"); btn.textContent = _("openSettings");
    btn.onclick = () => nativeRequest({ cmd: "open_settings" }).catch(() => {});
    b.appendChild(btn);
  }
}

// ---------- page ----------
async function currentTab() {
  const [t] = await browser.tabs.query({ active: true, windowId: state.windowId });
  return t;
}
async function showPage(tab) {
  if (!tab) return;
  state.tabId = tab.id;
  $("pageTitle").textContent = tab.title || tab.url || "";
  $("pageTitle").title = tab.url || "";
  const fav = $("favicon");
  if (tab.favIconUrl && /^(https?:|data:)/.test(tab.favIconUrl)) { fav.src = tab.favIconUrl; fav.hidden = false; } else fav.hidden = true;
  renderConversation();
  updateButtons();
}
async function readPage(tabId) {
  const tab = await browser.tabs.get(tabId);
  if (RESTRICTED.test(tab.url || "")) throw new Error(_("restricted"));
  try {
    const [res] = await browser.tabs.executeScript(tabId, { file: "/extract.js" });
    if (!res) throw new Error();
    return res;
  } catch (e) { throw new Error(_("restricted")); }
}

// ---------- conversation UI ----------
function renderConversation() {
  const log = $("log");
  log.querySelectorAll(".msg").forEach(n => n.remove());
  const c = conv(state.tabId);
  $("hint").hidden = c.nodes.length > 0;
  for (const n of c.nodes) log.appendChild(n);
  log.scrollTop = log.scrollHeight;
}
function addNode(cls, text) {
  const c = conv(state.tabId);
  const n = document.createElement("div"); n.className = "msg " + cls;
  if (text !== undefined) n.textContent = text;
  c.nodes.push(n); $("log").appendChild(n); $("hint").hidden = true;
  $("log").scrollTop = $("log").scrollHeight;
  return n;
}
function botTools(node, getText, pageInfo) {
  const tools = document.createElement("div"); tools.className = "tools";
  const copy = document.createElement("button"); copy.textContent = _("copy");
  copy.onclick = async () => { await navigator.clipboard.writeText(getText()); copy.textContent = _("copied"); setTimeout(() => copy.textContent = _("copy"), 1200); };
  const save = document.createElement("button"); save.textContent = _("saveNote");
  save.onclick = () => saveNote(getText(), pageInfo);
  tools.append(copy, save); node.appendChild(tools);
}
function setBusy(b) {
  state.busy = b;
  $("sendIcon").hidden = b; $("stopIcon").hidden = !b;
  $("send").title = b ? _("stop") : _("send"); $("send").setAttribute("aria-label", $("send").title);
  updateButtons();
}
function updateButtons() {
  const ok = state.status && state.status.ok;
  for (const id of ["btnSummarize", "btnForm"]) $(id).disabled = state.busy || !ok;
  $("btnNote").disabled = state.busy;
  $("send").disabled = !state.busy && (!ok || !$("input").value.trim());
}

async function ask(task, question, extra = {}) {
  if (state.busy) return null;
  const tabId = state.tabId, c = conv(tabId);
  addNode("user", task === "summarize" ? _("menuSummarize") : question);
  const bot = addNode("bot");
  const status = document.createElement("div"); status.className = "status"; status.textContent = _("reading"); bot.appendChild(status);
  const body = document.createElement("div"); bot.appendChild(body);
  setBusy(true);
  let text = "", page = null;
  try {
    page = await readPage(tabId);
    if (extra.selection) page.selection = extra.selection;
    status.textContent = _("thinking");
    let pendingRender = false;
    const res = await nativeRequest({ cmd: "chat", task, question, page, history: c.messages }, (chunk) => {
      text += chunk;
      if (!pendingRender) { pendingRender = true; requestAnimationFrame(() => { pendingRender = false; renderMarkdown(body, text); $("log").scrollTop = $("log").scrollHeight; }); }
    });
    text = res.text || text;
    status.remove(); renderMarkdown(body, text);
    c.messages.push({ role: "user", content: task === "summarize" ? "Riassumi la pagina." : question }, { role: "assistant", content: text });
    botTools(bot, () => text, page);
    return { text, page };
  } catch (e) {
    status.remove();
    if (e.stopped && text) { renderMarkdown(body, text); botTools(bot, () => text, page); addNode("info", _("stopped")); }
    else { bot.className = "msg err"; bot.textContent = e.stopped ? _("stopped") : e.message; }
    return null;
  } finally { setBusy(false); }
}

async function saveNote(text, page) {
  try {
    const tab = await browser.tabs.get(state.tabId);
    const res = await nativeRequest({ cmd: "save_note", title: (page && page.title) || tab.title || "Nota", url: (page && page.url) || tab.url, body: text });
    const n = addNode("info", _("noteSaved"));
    const open = document.createElement("button"); open.textContent = _("openFolder");
    open.onclick = () => nativeRequest({ cmd: "open_folder", path: res.folder }).catch(() => {});
    n.appendChild(open);
  } catch (e) { addNode("err", _("noteSaveFailed") + " " + e.message); }
}

// ---------- form filling (always previewed and confirmed; never submits) ----------
let formFields = [];
async function startForm() {
  if (state.busy) return;
  const panel = $("formPanel");
  try {
    const tab = await browser.tabs.get(state.tabId);
    if (RESTRICTED.test(tab.url || "")) throw new Error(_("restricted"));
    const [fields] = await browser.tabs.executeScript(state.tabId, { file: "/formscan.js" });
    formFields = fields || [];
  } catch (e) { addNode("err", e.message && e.message !== "" ? e.message : _("restricted")); return; }
  if (!formFields.length) { addNode("info", _("formNone")); return; }
  $("formIntro").textContent = _("formFound", [String(formFields.length)]);
  $("formPreview").hidden = true; $("formRows").textContent = ""; $("formData").hidden = false;
  $("formGo").textContent = _("formPropose"); $("formGo").dataset.step = "propose";
  panel.hidden = false; $("formData").focus();
}
async function formGo() {
  const go = $("formGo");
  if (go.dataset.step === "propose") {
    const data = $("formData").value.trim(); if (!data) { $("formData").focus(); return; }
    go.disabled = true; go.textContent = _("thinking");
    let values = {};
    try {
      const fields = formFields.map(({ id, label, type, name, autocomplete, options }) => ({ id, label, type, name, autocomplete, options }));
      const res = await nativeRequest({ cmd: "chat", task: "fill", question: data, fields }, () => {});
      const m = /\{[\s\S]*\}/.exec(res.text || ""); values = m ? JSON.parse(m[0]) : {};
    } catch (e) { addNode("err", e.message); go.disabled = false; go.textContent = _("formPropose"); return; }
    go.disabled = false;
    const rows = $("formRows"); rows.textContent = "";
    let count = 0;
    for (const f of formFields) {
      const v = values[f.id]; if (v === undefined || v === null || String(v).trim() === "") continue;
      count++;
      const row = document.createElement("div"); row.className = "frow";
      const cb = document.createElement("input"); cb.type = "checkbox"; cb.checked = true; cb.dataset.id = f.id;
      const lab = document.createElement("label"); lab.textContent = f.label || f.name || f.id; lab.title = lab.textContent;
      const inp = document.createElement("input"); inp.type = "text"; inp.value = String(v); inp.dataset.id = f.id;
      row.append(cb, lab, inp); rows.appendChild(row);
    }
    if (!count) { addNode("info", _("formNoValues")); go.textContent = _("formPropose"); return; }
    $("formPreview").hidden = false; $("formData").hidden = true;
    go.textContent = _("formApply"); go.dataset.step = "apply";
  } else {
    const values = {};
    for (const row of $("formRows").children) {
      const [cb, , inp] = row.children; if (cb.checked) values[inp.dataset.id] = inp.value;
    }
    const fill = (vals) => {
      let n = 0;
      for (const [id, v] of Object.entries(vals)) {
        const el = document.querySelector(`[data-vela-id="${CSS.escape(id)}"]`); if (!el) continue;
        if (el.tagName === "SELECT") {
          const o = [...el.options].find(o => o.text.trim().toLowerCase() === String(v).toLowerCase() || o.value === v); if (!o) continue;
          el.value = o.value;
        } else {
          const proto = el.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
          Object.getOwnPropertyDescriptor(proto, "value").set.call(el, v);
        }
        el.dispatchEvent(new Event("input", { bubbles: true })); el.dispatchEvent(new Event("change", { bubbles: true }));
        el.style.boxShadow = "0 0 0 2px #D4624A"; setTimeout(() => el.style.boxShadow = "", 2500); n++;
      }
      return n;
    };
    try {
      const [n] = await browser.tabs.executeScript(state.tabId, { code: `(${fill.toString()})(${JSON.stringify(values)})` });
      addNode("info", _("formDone", [String(n || 0)]));
      nativeRequest({ cmd: "log", action: "fill_form", summary: `${Object.keys(values).length} campi`, confirmed: true }).catch(() => {});
    } catch (e) { addNode("err", _("restricted")); }
    $("formPanel").hidden = true;
  }
}

// ---------- wiring ----------
$("btnSummarize").onclick = () => ask("summarize", "");
$("btnNote").onclick = async () => {
  const c = conv(state.tabId), last = [...c.messages].reverse().find(m => m.role === "assistant");
  if (last) return saveNote(last.content, null);
  if (state.status && state.status.ok) { const r = await ask("summarize", ""); if (r) saveNote(r.text, r.page); }
  else { const tab = await browser.tabs.get(state.tabId); saveNote(tab.url, null); }
};
$("btnForm").onclick = startForm;
$("formGo").onclick = formGo;
$("formCancel").onclick = () => { $("formPanel").hidden = true; };
$("newChat").onclick = () => { stop(); state.tabs.delete(state.tabId); renderConversation(); };
$("send").onclick = () => {
  if (state.busy) { stop(); return; }
  const q = $("input").value.trim(); if (!q) return;
  $("input").value = ""; autosize(); ask("ask", q);
};
const autosize = () => { const i = $("input"); i.style.height = "auto"; i.style.height = Math.min(i.scrollHeight, 140) + "px"; updateButtons(); };
$("input").addEventListener("input", autosize);
$("input").addEventListener("keydown", (e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); $("send").click(); } });

browser.tabs.onActivated.addListener((info) => { if (info.windowId === state.windowId) browser.tabs.get(info.tabId).then(showPage); });
browser.tabs.onUpdated.addListener((tabId, ch, tab) => { if (tabId === state.tabId && (ch.title || ch.favIconUrl || ch.url)) showPage(tab); });
browser.tabs.onRemoved.addListener((tabId) => state.tabs.delete(tabId));

async function takePending() {
  const p = await browser.runtime.sendMessage({ type: "takePending" }).catch(() => null);
  if (!p) return;
  if (p.tabId !== state.tabId) await showPage(await browser.tabs.get(p.tabId));
  if (p.action === "summarize") ask("summarize", "");
  else if (p.action === "askSelection") { $("input").value = ""; ask("ask", _("explainSel") + "\n«" + p.selection.slice(0, 500) + "»", { selection: p.selection }); }
}
browser.runtime.onMessage.addListener((m) => { if (m && m.type === "pending") takePending(); });
window.addEventListener("focus", refreshStatus);

(async () => {
  state.windowId = (await browser.windows.getCurrent()).id;
  await showPage(await currentTab());
  await refreshStatus();
  takePending();
})();
