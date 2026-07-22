(() => {
  "use strict";
  const navStyles = document.createElement("link");
  navStyles.rel = "stylesheet";
  navStyles.href = "/static/nav-state.css";
  document.head.append(navStyles);
  const menu = document.querySelector(".menu");
  const nav = document.querySelector("#nav");
  const syncActiveNavigation = () => {
    const currentPath = location.pathname.replace(/\/$/, "") || "/";
    document.querySelectorAll("#nav a").forEach((link) => {
      const linkPath = new URL(link.href).pathname.replace(/\/$/, "") || "/";
      const active = linkPath === "/" ? currentPath === "/" : currentPath === linkPath || currentPath.startsWith(`${linkPath}/`);
      link.classList.toggle("active", active);
      if (active) link.setAttribute("aria-current", "page");
      else link.removeAttribute("aria-current");
    });
  };
  syncActiveNavigation();
  menu?.addEventListener("click", () => {
    const open = menu.getAttribute("aria-expanded") !== "true";
    menu.setAttribute("aria-expanded", String(open));
    nav?.classList.toggle("open", open);
  });

  const sameOrigin = (a) =>
    a.origin === location.origin && !a.hash && !a.hasAttribute("download") && a.target !== "_blank";

  async function navigate(url, push = true) {
    try {
      const response = await fetch(url, { headers: { "X-Requested-With": "ItalyDevelopersNavigation" } });
      if (!response.ok) throw new Error("navigation failed");
      const doc = new DOMParser().parseFromString(await response.text(), "text/html");
      const next = doc.querySelector("#page");
      const current = document.querySelector("#page");
      if (!next || !current) throw new Error("invalid page");
      const swap = () => {
        current.replaceWith(next);
        document.title = doc.title;
        document.querySelector('meta[name="description"]')?.setAttribute("content", doc.querySelector('meta[name="description"]')?.content || "");
        if (push) history.pushState({}, "", url);
        syncActiveNavigation();
        scrollTo({ top: 0, behavior: "instant" });
        next.focus({ preventScroll: true });
      };
      document.startViewTransition ? document.startViewTransition(swap) : swap();
    } catch {
      location.href = url;
    }
  }

  document.addEventListener("click", (event) => {
    const reply = event.target.closest("[data-reply-to]");
    if (reply) {
      const form = document.querySelector("#comment-form");
      if (!form) return;
      form.querySelector("[data-parent-id]").value = reply.dataset.replyTo;
      const note = form.querySelector("[data-reply-note]");
      note.hidden = false;
      note.firstChild.textContent = `Replying to ${reply.dataset.replyAuthor}. `;
      form.scrollIntoView({ behavior: "smooth", block: "center" });
      form.querySelector("textarea").focus();
      return;
    }
    if (event.target.closest("[data-cancel-reply]")) {
      const form = document.querySelector("#comment-form");
      form.querySelector("[data-parent-id]").value = "";
      form.querySelector("[data-reply-note]").hidden = true;
      return;
    }
    const a = event.target.closest("a");
    if (!a || a.target === "_blank" || event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || !sameOrigin(a)) return;
    event.preventDefault();
    navigate(a.href);
  });
  const footerContact = document.querySelector(".footer-links > div:last-child");
  if (footerContact && !footerContact.querySelector(".social-links")) {
    const social = document.createElement("div");
    social.className = "social-links";
    social.setAttribute("aria-label", "Italy Developers social media");
    social.innerHTML = '<a href="/login">Member sign in →</a>';
    footerContact.appendChild(social);
    fetch("/api/social-links").then((response) => response.ok ? response.json() : {}).then((links) => {
      Object.entries(links).forEach(([name, href]) => {
        const link = document.createElement("a");
        link.href = href; link.target = "_blank"; link.rel = "noopener";
        link.textContent = `${name[0].toUpperCase()}${name.slice(1)} ↗`;
        social.insertBefore(link, social.lastElementChild);
      });
    }).catch(() => {});
  }
  addEventListener("popstate", () => navigate(location.href, false));
})();
