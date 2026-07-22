(() => {
  "use strict";
  const navStyles = document.createElement("link");
  navStyles.rel = "stylesheet";
  navStyles.href = "/static/nav-state.css";
  document.head.append(navStyles);
  const communityStyles = document.createElement("link");
  communityStyles.rel = "stylesheet";
  communityStyles.href = "/static/community-interactive.css";
  document.head.append(communityStyles);
  const mediaStyles = document.createElement("link");
  mediaStyles.rel = "stylesheet";
  mediaStyles.href = "/static/media-fixes.css";
  document.head.append(mediaStyles);
  const reactionStyles = document.createElement("link");
  reactionStyles.rel = "stylesheet";
  reactionStyles.href = "/static/reaction-state.css";
  document.head.append(reactionStyles);
  document.addEventListener("error", (event) => {
    const img = event.target;
    if (!(img instanceof HTMLImageElement) || img.dataset.fallbackApplied) return;
    const href = img.closest("a")?.getAttribute("href") || location.pathname;
    const match = href.match(/^\/(services|work|tech-stack|about|insights|blog)\/([^/?#]+)/);
    if (!match) return;
    img.dataset.fallbackApplied = "true";
    img.classList.add("image-load-failed");
    const fallbacks = {services:"/static/images/small-business-websites.png",work:"/static/images/generated/work-doappointment.webp","tech-stack":"/static/images/generated/tech-python.webp",about:"/static/images/generated/about-community.webp",insights:"/static/images/generated/blog-website-scope.webp",blog:"/static/images/generated/blog-website-scope.webp"};
    img.src = fallbacks[match[1]] || "/static/images/small-business-websites.png";
  }, true);
  const refreshGeneratedCovers = (root = document) => root.querySelectorAll('img[src^="/media/covers/"]').forEach((img) => {
    const url = new URL(img.src, location.origin);
    if (url.searchParams.get("v") !== "2") { url.searchParams.set("v", "2"); img.src = `${url.pathname}${url.search}`; }
  });
  const restorePendingComment = (root = document) => {
    const form = root.querySelector?.("#comment-form");
    if (!form || form.hasAttribute("data-requires-auth")) return;
    try {
      const pending = JSON.parse(sessionStorage.getItem("pendingBlogComment") || "null");
      if (!pending || pending.path !== location.pathname) return;
      form.querySelector('textarea[name="body"]').value = pending.body || "";
      form.querySelector("[data-parent-id]").value = pending.parentId || "";
      if (pending.parentId) {
        const comment = root.querySelector(`#comment-${pending.parentId}`);
        comment?.querySelector("[data-reply-slot]")?.append(form);
        const note = form.querySelector("[data-reply-note]");
        note.hidden = false;
        note.firstChild.textContent = `Replying to ${pending.author || "this comment"}. `;
      }
      sessionStorage.removeItem("pendingBlogComment");
      form.requestSubmit();
    } catch { sessionStorage.removeItem("pendingBlogComment"); }
  };
  const decorateComments = (root = document) => root.querySelectorAll?.(".comment").forEach((comment) => {
    const name = comment.querySelector("header strong")?.textContent?.trim() || "M";
    comment.querySelector("header")?.setAttribute("data-initial", name.charAt(0).toUpperCase());
  });
  refreshGeneratedCovers();
  restorePendingComment();
  decorateComments();
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
        refreshGeneratedCovers(next);
        restorePendingComment(next);
        decorateComments(next);
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
      reply.closest(".comment")?.querySelector("[data-reply-slot]")?.append(form);
      form.classList.add("inline-reply-form");
      form.scrollIntoView({ behavior: "smooth", block: "center" });
      form.querySelector("textarea").focus();
      return;
    }
    if (event.target.closest("[data-cancel-reply]")) {
      const form = document.querySelector("#comment-form");
      form.querySelector("[data-parent-id]").value = "";
      form.querySelector("[data-reply-note]").hidden = true;
      document.querySelector("[data-comment-form-home]")?.after(form);
      form.classList.remove("inline-reply-form");
      return;
    }
    if (event.target.closest("[data-auth-open]")) {
      document.querySelector("[data-auth-dialog]")?.showModal();
      return;
    }
    if (event.target.closest("[data-auth-close]")) {
      document.querySelector("[data-auth-dialog]")?.close();
      return;
    }
    const authTab = event.target.closest("[data-auth-tab]");
    if (authTab) {
      const mode = authTab.dataset.authTab;
      document.querySelectorAll("[data-auth-tab]").forEach((tab) => tab.classList.toggle("active", tab === authTab));
      document.querySelectorAll("[data-auth-panel]").forEach((panel) => panel.hidden = panel.dataset.authPanel !== mode);
      document.querySelector(`[data-auth-panel="${mode}"] input:not([type="hidden"])`)?.focus();
      return;
    }
    const a = event.target.closest("a");
    if (!a || a.target === "_blank" || event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || !sameOrigin(a)) return;
    event.preventDefault();
    navigate(a.href);
  });
  document.addEventListener("submit", (event) => {
    const likeForm = event.target.closest?.(".post-like, .comment>footer form");
    if (likeForm) {
      event.preventDefault();
      const button = likeForm.querySelector("button[type='submit'], button:not([type])");
      if (!button || button.disabled) return;
      button.disabled = true;
      button.setAttribute("aria-busy", "true");
      fetch(likeForm.action, {method:"POST",body:new URLSearchParams(new FormData(likeForm)),headers:{Accept:"application/json","X-Requested-With":"ItalyDevelopersReaction"}})
        .then((response) => { if (!response.ok) throw new Error("reaction failed"); return response.json(); })
        .then((data) => {
          const isPost = likeForm.classList.contains("post-like");
          button.textContent = isPost ? `♥ ${data.count} ${data.count === 1 ? "like" : "likes"}` : `♥ ${data.count} Like`;
          button.classList.toggle("liked", Boolean(data.active));
          button.setAttribute("aria-pressed", String(Boolean(data.active)));
        })
        .catch(() => likeForm.submit())
        .finally(() => { button.disabled = false; button.removeAttribute("aria-busy"); });
      return;
    }
    const form = event.target.closest?.("#comment-form[data-requires-auth]");
    if (!form) return;
    event.preventDefault();
    const body = form.querySelector('textarea[name="body"]');
    if (!body.reportValidity()) return;
    const parentId = form.querySelector("[data-parent-id]")?.value || "";
    const replyButton = parentId ? document.querySelector(`[data-reply-to="${parentId}"]`) : null;
    sessionStorage.setItem("pendingBlogComment", JSON.stringify({path:location.pathname,body:body.value,parentId,author:replyButton?.dataset.replyAuthor || ""}));
    const dialog = document.querySelector("[data-auth-dialog]");
    const context = dialog?.querySelector("[data-auth-context]");
    if (context) context.textContent = parentId ? "Sign in once and your reply will be posted under this comment." : "Sign in once and your comment will be posted automatically.";
    dialog?.showModal();
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
