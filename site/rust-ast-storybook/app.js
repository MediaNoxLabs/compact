(() => {
  "use strict";
  const data = window.STORYBOOK_DATA;
  const root = document.documentElement;
  const theme = document.querySelector("#theme-toggle");
  const storedTheme = localStorage.getItem("compact-story-theme");
  if (storedTheme === "light" || storedTheme === "dark") root.dataset.theme = storedTheme;
  const syncTheme = () => theme.setAttribute("aria-pressed", String(root.dataset.theme === "light"));
  syncTheme();
  theme.addEventListener("click", () => {
    root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
    localStorage.setItem("compact-story-theme", root.dataset.theme);
    syncTheme();
  });

  const exampleFields = {
    sourceLabel: document.querySelector("#example-source-label"),
    rustLabel: document.querySelector("#example-rust-label"),
    source: document.querySelector("#example-source-code"),
    rust: document.querySelector("#example-rust-code"),
    note: document.querySelector("#example-note")
  };
  document.querySelectorAll("[data-example]").forEach(button => button.addEventListener("click", () => {
    const example = data.examples[button.dataset.example];
    document.querySelectorAll("[data-example]").forEach(item => {
      const active = item === button;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-selected", String(active));
    });
    Object.entries(example).forEach(([field, value]) => { exampleFields[field].textContent = value; });
  }));

  document.querySelectorAll("[data-era]").forEach(button => button.addEventListener("click", () => {
    const era = button.dataset.era;
    document.querySelectorAll("[data-era]").forEach(item => {
      const active = item === button;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-pressed", String(active));
    });
    document.querySelectorAll("[data-era-card]").forEach(card => card.classList.toggle("is-hidden", era !== "all" && card.dataset.eraCard !== era));
  }));

  const stageFields = {
    kicker: document.querySelector("#stage-kicker"),
    owner: document.querySelector("#stage-owner"),
    crosses: document.querySelector("#stage-crosses"),
    limit: document.querySelector("#stage-limit")
  };
  document.querySelectorAll("[data-stage]").forEach(button => button.addEventListener("click", () => {
    const key = button.dataset.stage;
    document.querySelectorAll(".stage-tab").forEach(item => {
      const active = item === button;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-selected", String(active));
    });
    document.querySelectorAll("[data-node]").forEach(node => node.classList.toggle("selected", node.dataset.node === key));
    Object.entries(data.stages[key]).forEach(([field, value]) => { stageFields[field].textContent = value; });
  }));

  const comparisonGrid = document.querySelector("#comparison-grid");
  const renderComparison = key => {
    comparisonGrid.replaceChildren(...data.comparisons[key].map((entry, index) => {
      const card = document.createElement("article");
      card.className = "comparison-card";
      const ordinal = document.createElement("span"); ordinal.className = "ordinal"; ordinal.textContent = String(index + 1).padStart(2, "0");
      const title = document.createElement("h3"); title.textContent = entry[0];
      const copy = document.createElement("p"); copy.textContent = entry[1];
      const verdict = document.createElement("span"); verdict.className = "verdict"; verdict.textContent = entry[2];
      card.append(ordinal, title, copy, verdict);
      return card;
    }));
  };
  renderComparison("boundary");
  document.querySelectorAll("[data-dimension]").forEach(button => button.addEventListener("click", () => {
    document.querySelectorAll("[data-dimension]").forEach(item => { const active = item === button; item.classList.toggle("is-active", active); item.setAttribute("aria-selected", String(active)); });
    renderComparison(button.dataset.dimension);
  }));

  const evidenceFields = ["#evidence-index", "#evidence-name", "#evidence-establishes", "#evidence-limit"].map(selector => document.querySelector(selector));
  document.querySelectorAll("[data-evidence]").forEach(button => button.addEventListener("click", () => {
    document.querySelectorAll("[data-evidence]").forEach(item => { const active = item === button; item.classList.toggle("is-active", active); item.setAttribute("aria-pressed", String(active)); });
    data.evidence[button.dataset.evidence].forEach((value, index) => { evidenceFields[index].textContent = value; });
  }));

  const decisionGrid = document.querySelector("#decision-grid");
  decisionGrid.replaceChildren(...data.decisions.map(entry => {
    const card = document.createElement("details"); card.className = "decision-card";
    const summary = document.createElement("summary");
    const number = document.createElement("span"); number.textContent = entry[0];
    const title = document.createElement("b"); title.textContent = entry[1];
    const marker = document.createElement("i"); marker.textContent = "+"; marker.setAttribute("aria-hidden", "true");
    summary.append(number, title, marker);
    const body = document.createElement("div"); body.className = "decision-body";
    const copy = document.createElement("p"); copy.textContent = entry[2]; body.append(copy);
    card.append(summary, body);
    card.addEventListener("toggle", () => { marker.textContent = card.open ? "−" : "+"; });
    return card;
  }));

  const navLinks = [...document.querySelectorAll(".topbar nav a")];
  const observed = navLinks.map(link => document.querySelector(link.getAttribute("href"))).filter(Boolean);
  const observer = new IntersectionObserver(entries => entries.forEach(entry => {
    if (!entry.isIntersecting) return;
    navLinks.forEach(link => link.classList.toggle("is-current", link.getAttribute("href") === `#${entry.target.id}`));
  }), { rootMargin: "-25% 0px -65%", threshold: 0 });
  observed.forEach(section => observer.observe(section));
})();
