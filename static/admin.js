(() => {
  "use strict";
  const $ = (s, c = document) => c.querySelector(s),
    $$ = (s, c = document) => [...c.querySelectorAll(s)];
  // htmx's outerHTML swap removes the checked <input> and inserts a fresh one,
  // so the browser drops focus to <body> and jumps the scroll position. Put
  // focus back on the new switch without letting that focus() call re-scroll.
  document.body.addEventListener("htmx:afterSettle", (event) => {
    const swapped = event.target;
    if (swapped instanceof HTMLElement && swapped.matches(".table-switch")) {
      swapped.querySelector("input")?.focus({ preventScroll: true });
    }
  });
  const adminFixes = document.createElement("link");
  adminFixes.rel = "stylesheet";
  adminFixes.href = "/static/admin-fixes.css?v=2";
  document.head.append(adminFixes);
  const toast = (message, type = "success") => {
    const stack = $(".toast-stack");
    if (!stack) return;
    const el = document.createElement("div");
    el.className = `toast ${type}`;
    el.textContent = message;
    stack.append(el);
    setTimeout(() => el.remove(), 4200);
  };
  const toastKey = document.body.dataset.toast;
  if (toastKey) {
    const messages = {
      welcome: "Welcome back. Your workspace is ready.",
      saved: "Content saved successfully.",
      deleted: "Content deleted.",
      "lead-updated": "Enquiry status updated.",
      "lead-deleted": "Enquiry deleted.",
    };
    toast(messages[toastKey] || "Done.");
  }
  const passwordToggle = $("[data-password-toggle]");
  passwordToggle?.addEventListener("click", () => {
    const input = $("#password");
    const show = input.type === "password";
    input.type = show ? "text" : "password";
    passwordToggle.textContent = show ? "Hide" : "Show";
  });
  const loginForm = $("[data-validate]");
  loginForm?.addEventListener("submit", (event) => {
    let valid = true;
    loginForm.querySelectorAll("input").forEach((input) => {
      const field = input.closest(".field"),
        error = $(".field-error", field);
      field.classList.remove("has-error");
      error.textContent = "";
      if (!input.validity.valid) {
        valid = false;
        field.classList.add("has-error");
        error.textContent =
          input.type === "email"
            ? "Enter a valid work email."
            : "Password must contain at least 12 characters.";
      }
    });
    if (!valid) event.preventDefault();
  });
  const panelButtons = $$("[data-panel-button]");
  panelButtons.forEach((button) =>
    button.addEventListener("click", () => {
      panelButtons.forEach((v) => v.classList.toggle("active", v === button));
      $$("[data-panel]").forEach((panel) =>
        panel.classList.toggle(
          "active",
          panel.dataset.panel === button.dataset.panelButton,
        ),
      );
      const crumb = $("[data-breadcrumb]");
      if (crumb)
        crumb.textContent =
          button.dataset.panelButton === "leads" ? "Enquiries" : "Content";
      document.querySelector(".sidebar")?.classList.remove("open");
    }),
  );
  const sidebarNav = $(".sidebar nav");
  if (sidebarNav && $('[data-panel-button="leads"]')) {
    const homeControl = document.createElement("a");
    homeControl.className = "nav-item";
    homeControl.href = "/admin/homepage";
    homeControl.innerHTML = "<span>⌂</span>Homepage <b>Control</b>";
    sidebarNav.insertBefore(homeControl, sidebarNav.lastElementChild);
  }
  $("[data-sidebar-toggle]")?.addEventListener("click", () =>
    $(".sidebar")?.classList.toggle("open"),
  );
  const earlyFilterBox = $("[data-kind-filter]");
  if (
    earlyFilterBox &&
    !earlyFilterBox.querySelector('[data-kind="testimonial"]')
  ) {
    const testimonialButton = document.createElement("button");
    testimonialButton.type = "button";
    testimonialButton.dataset.kind = "testimonial";
    testimonialButton.textContent = "Testimonials";
    earlyFilterBox.append(testimonialButton);
  }
  const pageMap = {
    service: {
      label: "Services page",
      route: "services",
      extra: "Home · Services section",
    },
    work: { label: "Work page", route: "work", extra: "Home · Selected work" },
    blog: { label: "Blog page", route: "blog", extra: "Home · Ideas section" },
    insight: {
      label: "Insights page",
      route: "insights",
      extra: "Home · Ideas section",
    },
    tech: {
      label: "Tech stack page",
      route: "tech-stack",
      extra: "Tech stack listing",
    },
    about: { label: "About page", route: "about", extra: "About listing" },
  };
  const filterBox = $("[data-kind-filter]");
  if (filterBox && !filterBox.querySelector('[data-kind="about"]')) {
    const aboutButton = document.createElement("button");
    aboutButton.type = "button";
    aboutButton.dataset.kind = "about";
    aboutButton.textContent = "About";
    filterBox.append(aboutButton);
  }
  $$("[data-content-row]").forEach((row) => {
    const meta = row.querySelector(".content-identity span"),
      map = pageMap[row.dataset.kind];
    if (!meta || !map) return;
    const slug = (meta.textContent.split("·")[0] || "")
      .trim()
      .replace(/^\//, "");
    meta.classList.add("content-location");
    meta.textContent = "";
    const badge = document.createElement("b");
    badge.className = "page-badge";
    badge.textContent = map.label;
    const path = document.createElement("span");
    path.className = "content-path";
    path.textContent = `/${map.route}/${slug}`;
    const section = document.createElement("span");
    section.className = "content-path";
    section.textContent = `Also: ${map.extra}`;
    meta.append(badge, path, section);
  });
  const contentList = $(".content-list");
  const resultText = document.createElement("p");
  resultText.className = "filter-result";
  contentList?.before(resultText);
  let currentKind = "all";
  const applyFilters = () => {
    const query = ($("[data-content-search]")?.value || "")
      .trim()
      .toLowerCase();
    let visible = 0;
    $$("[data-content-row]").forEach((row) => {
      const show =
        (currentKind === "all" || row.dataset.kind === currentKind) &&
        row.dataset.search.toLowerCase().includes(query);
      row.hidden = !show;
      if (show) visible++;
    });
    $("[data-empty-state]")?.classList.toggle("show", visible === 0);
    const label =
      currentKind === "all"
        ? "all content"
        : `${pageMap[currentKind]?.label || currentKind}`;
    resultText.innerHTML = `Showing <strong>${visible}</strong> item${visible === 1 ? "" : "s"} for <strong>${label}</strong>${query ? ` matching “${query.replace(/[<>]/g, "")}”` : ""}`;
  };
  $("[data-content-search]")?.addEventListener("input", applyFilters);
  $$("[data-kind]").forEach((button) =>
    button.addEventListener("click", () => {
      const alreadyActive = button.classList.contains("active");
      currentKind =
        button.dataset.kind !== "all" && alreadyActive
          ? "all"
          : button.dataset.kind;
      $$("[data-kind]").forEach((v) =>
        v.classList.toggle("active", v.dataset.kind === currentKind),
      );
      applyFilters();
    }),
  );
  applyFilters();
  const dialog = $("[data-confirm-dialog]");
  let pendingForm = null;
  $$("[data-confirm-form]").forEach((button) =>
    button.addEventListener("click", () => {
      pendingForm = document.getElementById(button.dataset.confirmForm);
      dialog?.showModal();
    }),
  );
  $("[data-dialog-cancel]")?.addEventListener("click", () => dialog.close());
  $("[data-dialog-confirm]")?.addEventListener("click", () =>
    pendingForm?.submit(),
  );
  pageMap.testimonial = {
    label: "Homepage testimonials",
    route: "#testimonials",
    extra: "Home · Client words",
  };
  $$(
    '[data-content-row][data-kind="testimonial"] .content-identity span',
  ).forEach((meta) => {
    meta.classList.add("content-location");
    meta.innerHTML =
      '<b class="page-badge">Homepage testimonials</b><span class="content-path">/#testimonials</span><span class="content-path">Quote shown directly on Home</span>';
  });
  const form = $("[data-content-form]"),
    visual = $("[data-visual-editor]"),
    source = $("[data-source-editor]");
  if (!form || !visual || !source) return;
  const kindHelp = {
    service: {
      name: "Service",
      prefix: "/services/",
      location:
        "Services listing, homepage Services section and its own detail page.",
      title: "Service name",
      eyebrow: "Service category",
      summary: "Service card summary",
      body: "Full service page content",
      cta: "Request this service",
    },
    work: {
      name: "Work / case study",
      prefix: "/work/",
      location:
        "Work listing, homepage Selected Work section and its own case-study page.",
      title: "Project or client name",
      eyebrow: "Industry / project type",
      summary: "Outcome shown on the project card",
      body: "Challenge, solution and results",
      cta: "Discuss a similar project",
    },
    tech: {
      name: "Tech stack",
      prefix: "/tech-stack/",
      location: "Tech Stack listing and its own technology detail page.",
      title: "Technology name",
      eyebrow: "Technology category",
      summary: "Why this technology is useful",
      body: "Benefits, use cases and technical details",
      cta: "Discuss the technology",
    },
    about: {
      name: "About section",
      prefix: "/about/",
      location:
        "Rendered directly on the About page and also available as its own detail page.",
      title: "About section heading",
      eyebrow: "Section label",
      summary: "Short introduction beside the section",
      body: "Main About content shown directly on /about",
      cta: "Start a conversation",
    },
    insight: {
      name: "Insight",
      prefix: "/insights/",
      location:
        "Insights listing, homepage Ideas section and its own article page.",
      title: "Insight headline",
      eyebrow: "Topic",
      summary: "Article card introduction",
      body: "Complete insight article",
      cta: "Discuss this topic",
    },
    blog: {
      name: "Blog post",
      prefix: "/blog/",
      location:
        "Blog listing, homepage Ideas section and its own article page.",
      title: "Blog headline",
      eyebrow: "Blog category",
      summary: "Blog card introduction",
      body: "Complete blog article",
      cta: "Talk to us",
    },
    testimonial: {
      name: "Testimonial",
      prefix: "/#testimonials",
      fieldPrefix: "testimonial/",
      direct: true,
      location: "Shown directly in the homepage Client Words section.",
      title: "Client name",
      eyebrow: "Business · city",
      summary: "Customer quote",
      body: "Optional internal notes",
      cta: "Project or result",
      slugLabel: "Internal identifier",
    },
  };
  const kindSelect = $("#kind"),
    guide = document.createElement("div");
  if (!kindSelect.querySelector('option[value="testimonial"]')) {
    const option = document.createElement("option");
    option.value = "testimonial";
    option.textContent = "Testimonial";
    kindSelect.append(option);
  }
  guide.className = "content-type-guide";
  guide.innerHTML =
    '<span>?</span><div><strong data-kind-name></strong><small data-kind-location></small></div><a data-public-preview target="_blank">View page ↗</a>';
  $(".editor-layout")?.prepend(guide);
  const setLabel = (id, text) => {
    const label = document.querySelector(`label[for="${id}"]`);
    if (label) label.textContent = text;
  };
  const applyKindHelp = () => {
    const config = kindHelp[kindSelect.value] || kindHelp.service;
    $("[data-kind-name]", guide).textContent = `Editing: ${config.name}`;
    $("[data-kind-location]", guide).textContent = config.location;
    const preview = $("[data-public-preview]", guide);
    preview.href = config.direct
      ? config.prefix
      : config.prefix + (form.elements.slug.value || "");
    setLabel("title", config.title);
    setLabel("eyebrow", config.eyebrow);
    setLabel("summary", config.summary);
    setLabel("slug", config.slugLabel || "URL slug");
    setLabel("cta", "Call-to-action button");
    const bodyHeading = $$(".form-section h2")[1];
    if (bodyHeading) bodyHeading.textContent = config.body;
    const prefix = $(".slug-input > span");
    if (prefix) prefix.textContent = config.fieldPrefix || config.prefix;
    form.elements.cta.placeholder = config.cta;
    form.elements.summary.placeholder = `Write the short text used for ${config.name.toLowerCase()} previews.`;
  };
  kindSelect.addEventListener("change", applyKindHelp);
  form.elements.slug.addEventListener("input", applyKindHelp);
  const requestedType = new URLSearchParams(location.search).get("type");
  if (requestedType && kindHelp[requestedType]) kindSelect.value = requestedType;
  const featuredTitle = $$(".switch-row strong").find(
    (element) => element.textContent.trim() === "Featured",
  );
  if (featuredTitle) {
    featuredTitle.textContent = "Show on homepage";
    const featuredHelp = featuredTitle.parentElement.querySelector("span");
    if (featuredHelp)
      featuredHelp.textContent = "Homepage settings control the maximum shown";
  }
  applyKindHelp();
  const syncSource = () => (source.value = visual.innerHTML),
    syncVisual = () => (visual.innerHTML = source.value);
  const imageCommandInput = $("[data-image-command-input]");
  let savedRange = null;
  $$("[data-command]").forEach((button) =>
    button.addEventListener("click", () => {
      if (button.dataset.command === "image") {
        const selection = window.getSelection();
        savedRange = selection && selection.rangeCount ? selection.getRangeAt(0) : null;
        imageCommandInput?.click();
        return;
      }
      let value = button.dataset.value || null;
      if (button.dataset.command === "createLink") {
        value = prompt("Enter a secure HTTPS link") || "";
        if (value && !value.startsWith("https://")) {
          toast("Links must start with https://", "error");
          return;
        }
      }
      visual.focus();
      document.execCommand(button.dataset.command, false, value);
      syncSource();
      updateProgress();
    }),
  );
  imageCommandInput?.addEventListener("change", async () => {
    const file = imageCommandInput.files[0];
    imageCommandInput.value = "";
    if (!file) return;
    if (file.size > 8 * 1024 * 1024) {
      toast("Image must be smaller than 8 MB.", "error");
      return;
    }
    const body = new FormData();
    body.append("image", file, file.name);
    visual.focus();
    if (savedRange) {
      const selection = window.getSelection();
      selection.removeAllRanges();
      selection.addRange(savedRange);
    }
    try {
      const response = await fetch("/admin/media/upload", { method: "POST", body });
      if (!response.ok) throw new Error("upload failed");
      const data = await response.json();
      document.execCommand("insertImage", false, data.url);
      syncSource();
      updateProgress();
      markDirty();
    } catch {
      toast("Image upload failed.", "error");
    }
  });
  $("[data-source-toggle]")?.addEventListener("click", (event) => {
    const editor = event.currentTarget.closest(".rich-editor"),
      active = editor.classList.toggle("source-mode");
    if (active) syncSource();
    else syncVisual();
    event.currentTarget.textContent = active ? "Visual" : "HTML";
  });
  visual.addEventListener("input", () => {
    syncSource();
    validateField(source);
    updateProgress();
    markDirty();
  });
  source.addEventListener("input", () => {
    syncVisual();
    validateField(source);
    updateProgress();
    markDirty();
  });
  const title = $("[data-slug-source]"),
    slug = $("[data-slug-target]");
  title?.addEventListener("input", () => {
    if (!slug.dataset.touched) {
      slug.value = title.value
        .toLowerCase()
        .normalize("NFD")
        .replace(/[\u0300-\u036f]/g, "")
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
      slug.dispatchEvent(new Event("input"));
    }
  });
  slug?.addEventListener("input", () => {
    slug.dataset.touched = "1";
    const preview = $("[data-preview-path]");
    if (preview) preview.textContent = slug.value || "page-url";
  });
  $$("[data-counter]").forEach((counter) => {
    const input = document.getElementById(counter.dataset.counter);
    const render = () => {
      counter.textContent = `${input.value.length} / ${input.maxLength}`;
      counter.classList.toggle(
        "limit",
        input.value.length > input.maxLength * 0.9,
      );
    };
    input.addEventListener("input", render);
    render();
  });
  const seoTitle = $("#seo_title"),
    seoDescription = $("#seo_description");
  seoTitle?.addEventListener(
    "input",
    () =>
      ($("[data-preview-title]").textContent =
        seoTitle.value || "Your search title"),
  );
  seoDescription?.addEventListener(
    "input",
    () =>
      ($("[data-preview-description]").textContent =
        seoDescription.value || "Your search description will appear here."),
  );
  const imageInput = $("[data-image-input]");
  imageInput?.addEventListener("change", () => {
    const file = imageInput.files[0];
    if (!file) return;
    if (file.size > 8 * 1024 * 1024) {
      imageInput.value = "";
      toast("Image must be smaller than 8 MB.", "error");
      return;
    }
    const preview = $("[data-image-preview]"),
      empty = $("[data-image-empty]");
    preview.src = URL.createObjectURL(file);
    preview.hidden = false;
    empty.hidden = true;
    markDirty();
  });
  let required = [
    "title",
    "slug",
    "summary",
    "body",
    "seo_title",
    "seo_description",
  ];
  const configureEditorFields = () => {
    const testimonial = kindSelect.value === "testimonial";
    required = testimonial
      ? ["title", "slug", "summary"]
      : ["title", "slug", "summary", "body", "seo_title", "seo_description"];
    const sections = $$(".form-section");
    if (sections[1]) sections[1].hidden = testimonial;
    if (sections[2]) sections[2].hidden = testimonial;
    setLabel("cta", testimonial ? "Project / result (optional)" : "Call-to-action button");
    const completionHelp = $(".completion-card small");
    if (completionHelp)
      completionHelp.textContent = testimonial
        ? "Add the client, business and quote; then publish and show it on Home."
        : "Complete required fields and SEO metadata.";
    updateProgress();
  };
  kindSelect.addEventListener("change", configureEditorFields);
  const validateField = (input) => {
    const field = input.closest(".field") || input.closest(".rich-editor"),
      error =
        field?.parentElement?.querySelector(":scope > .field-error") ||
        field?.querySelector(".field-error");
    let message = "";
    if (
      input.name === "body" &&
      source.value.replace(/<[^>]*>/g, "").trim().length < 20
    )
      message = "Main content must contain at least 20 characters.";
    else if (!input.validity.valid) {
      if (input.validity.valueMissing) message = "This field is required.";
      else if (input.validity.tooShort)
        message = `Use at least ${input.minLength} characters.`;
      else if (input.validity.tooLong)
        message = `Keep this under ${input.maxLength} characters.`;
      else if (input.validity.patternMismatch)
        message = "Use lowercase letters, numbers and hyphens only.";
    }
    field?.classList.toggle("has-error", !!message);
    if (error) error.textContent = message;
    return !message;
  };
  const updateProgress = () => {
    syncSource();
    let completed = 0;
    required.forEach((name) => {
      const input = form.elements[name];
      if (
        input &&
        (name === "body"
          ? source.value.replace(/<[^>]*>/g, "").trim().length >= 20
          : input.validity.valid)
      )
        completed++;
    });
    const percent = Math.round((completed / required.length) * 100);
    $("[data-completion]").textContent = `${percent}%`;
    $("[data-completion-bar]").value = percent;
  };
  configureEditorFields();
  const validateForm = () => {
    updateProgress();
    return required.every((name) => validateField(form.elements[name]));
  };
  form.addEventListener("focusout", (event) => {
    if (event.target.matches("input,textarea,select"))
      validateField(event.target);
  });
  form.addEventListener("submit", (event) => {
    syncSource();
    if (!validateForm()) {
      event.preventDefault();
      toast("Please fix the highlighted fields before saving.", "error");
      form
        .querySelector(".has-error")
        ?.scrollIntoView({ behavior: "smooth", block: "center" });
    } else {
      sessionStorage.removeItem("italy-cms-draft");
      $("[data-save-state]").textContent = "Saving…";
    }
  });
  const markDirty = () => {
    $("[data-save-state]").textContent = "Unsaved changes";
  };
  form.addEventListener("input", () => {
    markDirty();
    updateProgress();
  });
  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      form.requestSubmit();
    }
  });
  updateProgress();
})();
