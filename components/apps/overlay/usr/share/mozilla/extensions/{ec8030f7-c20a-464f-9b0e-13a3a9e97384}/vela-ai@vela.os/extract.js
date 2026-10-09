// Injected into the page on demand (user pressed a button). Returns readable text; never modifies the page.
(() => {
  const pick = document.querySelector("article") || document.querySelector("main, [role=main]") || document.body;
  let text = "";
  try {
    const clone = pick.cloneNode(true);
    clone.querySelectorAll("script,style,noscript,svg,nav,footer,header,aside,form,iframe,[aria-hidden=true]").forEach(n => n.remove());
    document.body.appendChild(clone); clone.style.cssText = "position:absolute;left:-99999px;top:0;width:800px";
    text = clone.innerText; clone.remove();
  } catch (e) { text = (pick || document.body).innerText || ""; }
  if (text.trim().length < 200) text = document.body.innerText || text;
  text = text.replace(/[ \t]+/g, " ").replace(/\n{3,}/g, "\n\n").trim();
  return { title: document.title, url: location.href, lang: document.documentElement.lang || "",
           selection: String(window.getSelection() || "").slice(0, 4000), text: text.slice(0, 20000) };
})();
