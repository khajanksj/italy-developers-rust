(() => {
  "use strict";
  document.querySelectorAll("[data-lang-tab]").forEach((btn) => {
    btn.addEventListener("click", () => {
      document
        .querySelectorAll("[data-lang-tab]")
        .forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const code = btn.getAttribute("data-lang-tab");
      document.querySelectorAll("[data-lang-panel]").forEach((p) => {
        p.hidden = p.getAttribute("data-lang-panel") !== code;
      });
    });
  });
})();
