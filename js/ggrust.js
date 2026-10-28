// ggrust interactive runtime, embedded in every HTML output by the Rust renderer
// (crates/ggrust-core/src/render/html.rs).
//
// What it does: defines `window.ggrust` once per page (replaced when a different
// ggrust version renders on the same page) and mounts each plot by the id of its
// root element. The SVG itself is complete without this script: notebooks that do
// not run JavaScript (GitHub, nbviewer) still show the static plot.
// What it does not do (yet): tooltips (plan.md 4.2), zoom/pan (4.3), canvas
// point layers (4.4). Those attach to the mounted plot object.
//
// The file is one function expression; the renderer calls it with the engine
// version. It is inlined in a script element, so it must never contain a
// less-than sign followed by a slash, or by an exclamation mark and two dashes.
(function (version) {
  "use strict";
  if (window.ggrust && window.ggrust.version === version) {
    return;
  }

  // Attach to the plot whose root element has id `id`. Mounting twice returns
  // the same object. The plot lives on its root element, so removing the output
  // from the page frees it (no page-wide registry).
  function mount(id) {
    const root = document.getElementById(id);
    if (root === null) {
      return null;
    }
    if (root.ggrust) {
      return root.ggrust;
    }
    const plot = { id: id, root: root, svg: root.querySelector("svg") };
    root.ggrust = plot;
    root.setAttribute("data-ggrust-mounted", version);
    return plot;
  }

  window.ggrust = { version: version, mount: mount };
})
