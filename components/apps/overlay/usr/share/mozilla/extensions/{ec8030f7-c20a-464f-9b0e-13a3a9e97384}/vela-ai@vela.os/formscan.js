// Lists fillable form fields (no passwords, payment, hidden or file fields) and tags them with data-vela-id.
(() => {
  const BAD = /(pass(word)?|pwd|card|cc-|cvv|cvc|iban|pin|otp|securitycode|expir)/i;
  const out = []; let n = 0;
  const labelOf = (el) => {
    if (el.labels && el.labels.length) return [...el.labels].map(l => l.innerText).join(" ").trim();
    const a = el.getAttribute("aria-label") || ""; if (a) return a;
    const by = el.getAttribute("aria-labelledby"); if (by) { const t = by.split(/\s+/).map(i => document.getElementById(i)?.innerText || "").join(" ").trim(); if (t) return t; }
    return el.placeholder || el.name || el.id || "";
  };
  for (const el of document.querySelectorAll("input, textarea, select")) {
    const type = (el.type || "text").toLowerCase();
    if (["hidden","password","file","submit","button","image","reset","checkbox","radio","range","color"].includes(type)) continue;
    if (el.disabled || el.readOnly) continue;
    const r = el.getBoundingClientRect(); if (!r.width || !r.height || getComputedStyle(el).visibility === "hidden") continue;
    const ac = el.getAttribute("autocomplete") || "";
    const label = labelOf(el).replace(/\s+/g, " ").slice(0, 120);
    if (BAD.test(ac) || BAD.test(el.name || "") || BAD.test(el.id || "") || BAD.test(label)) continue;
    const id = el.dataset.velaId || ("f" + (++n) + "_" + Math.random().toString(36).slice(2, 6));
    el.dataset.velaId = id;
    const f = { id, label, type: el.tagName === "SELECT" ? "select" : el.tagName === "TEXTAREA" ? "textarea" : type,
                name: el.name || "", autocomplete: ac, current: (el.value || "").slice(0, 200) };
    if (el.tagName === "SELECT") f.options = [...el.options].slice(0, 40).map(o => o.text.trim()).filter(Boolean);
    out.push(f);
    if (out.length >= 60) break;
  }
  return out;
})();
