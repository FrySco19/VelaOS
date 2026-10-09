"use strict";
// Minimal, safe Markdown → DOM (no innerHTML from model output): paragraphs, bullet/numbered lists, **bold**, `code`.
function renderMarkdown(el, text) {
  el.textContent = "";
  const lines = String(text).replace(/\r/g, "").split("\n");
  let list = null, para = [];
  const flushPara = () => { if (para.length) { const p = document.createElement("p"); inline(p, para.join(" ")); el.appendChild(p); para = []; } };
  for (const raw of lines) {
    const line = raw.trimEnd();
    const ul = /^\s*[-*•]\s+(.*)$/.exec(line), ol = /^\s*\d+[.)]\s+(.*)$/.exec(line), h = /^#{1,6}\s+(.*)$/.exec(line);
    if (ul || ol) {
      flushPara();
      const tag = ul ? "UL" : "OL";
      if (!list || list.tagName !== tag) { list = document.createElement(tag); el.appendChild(list); }
      const li = document.createElement("li"); inline(li, (ul || ol)[1]); list.appendChild(li);
    } else if (!line.trim()) { flushPara(); list = null; }
    else if (h) { flushPara(); list = null; const p = document.createElement("p"); const b = document.createElement("strong"); inline(b, h[1]); p.appendChild(b); el.appendChild(p); }
    else { list = null; para.push(line.trim()); }
  }
  flushPara();
}
function inline(parent, s) {
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g; let last = 0, m;
  while ((m = re.exec(s))) {
    if (m.index > last) parent.appendChild(document.createTextNode(s.slice(last, m.index)));
    const t = m[0], node = document.createElement(t.startsWith("**") ? "strong" : "code");
    node.textContent = t.startsWith("**") ? t.slice(2, -2) : t.slice(1, -1);
    parent.appendChild(node); last = m.index + t.length;
  }
  if (last < s.length) parent.appendChild(document.createTextNode(s.slice(last)));
}
