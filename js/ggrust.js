// ggrust interactive runtime, embedded in every HTML output by the Rust renderer
// (crates/ggrust-core/src/render/html.rs).
//
// What it does: defines `window.ggrust` once per page (replaced when a different
// ggrust version renders on the same page) and mounts each plot by the id of its
// root element. Mounting reads the tooltip payload the engine wrote next to the
// SVG (targets in scene pixels and every row's text, see crates/ggrust-core/src/
// tooltip), buckets the targets into a grid, and shows lets-plot-style tooltips
// on hover: general tooltips (one box per hit target) and axis tooltips. All text
// is computed by the engine; this file only looks targets up and lays boxes out.
// The SVG is complete without this script: notebooks that do not run JavaScript
// (GitHub, nbviewer) still show the static plot.
// What it does not do (yet): zoom/pan (plan.md 4.3), canvas point layers (4.4).
//
// The file is one function expression; the renderer calls it with the engine
// version. It is inlined in a script element, so it must never contain a
// less-than sign followed by a slash, or by an exclamation mark and two dashes.
(function (version) {
  "use strict";
  if (window.ggrust && window.ggrust.version === version) {
    return;
  }

  // lets-plot constants (TooltipDefaults, PointTargetProjection, LocatedTargetsPicker).
  var POINT_REACH = 5.1; // px beyond a point's radius
  var CUTOFF = 30; // px: nearest targets farther away are ignored
  var AREA_DISTANCE = 15; // rects and polygons under the cursor rank behind close points
  var CELL = 32; // px per grid cell of the hit index
  var GAP = 5; // px between stacked tooltips
  var OFFSET = 8; // px between a tooltip and its target
  var TEXT = "#474747";
  var AXIS_FILL = "#3d3d3d";
  var FONT = "13px Lato, 'Helvetica Neue', Helvetica, Arial, sans-serif";

  // A dictionary-encoded text column: the text of `row`, or null.
  function text(column, row) {
    if (!column) {
      return null;
    }
    var code = column.codes[row];
    return code === undefined || code < 0 ? null : column.values[code];
  }

  // Target `index` of a layer as {x0, y0, x1, y1} (its reach included), and the
  // point a tooltip points at.
  function extent(layer, index) {
    var t = layer.targets;
    switch (t.kind) {
      case "points": {
        var reach = t.radius[index] + POINT_REACH;
        return [t.x[index] - reach, t.y[index] - reach, t.x[index] + reach, t.y[index] + reach];
      }
      case "vertices":
        return [
          t.x[index] - POINT_REACH,
          t.y[index] - POINT_REACH,
          t.x[index] + POINT_REACH,
          t.y[index] + POINT_REACH,
        ];
      case "rects":
        return [t.x[index], t.y[index], t.x[index] + t.width[index], t.y[index] + t.height[index]];
      default: {
        var ring = t.rings[index];
        var box = [Infinity, Infinity, -Infinity, -Infinity];
        for (var i = 0; i + 1 < ring.length; i += 2) {
          box[0] = Math.min(box[0], ring[i]);
          box[1] = Math.min(box[1], ring[i + 1]);
          box[2] = Math.max(box[2], ring[i]);
          box[3] = Math.max(box[3], ring[i + 1]);
        }
        return box;
      }
    }
  }

  function count(layer) {
    return layer.targets.rows.length;
  }

  // Grid buckets of every target's extent, clipped to its panel (the cursor is
  // looked up inside the panel only, and a zoomed-in shape can be millions of
  // pixels wide); along-x lookups bucket by column.
  function index(layer) {
    var cells = new Map();
    var alongX = layer.lookup === "hover_x";
    var p = layer.panel;
    for (var i = 0; i < count(layer); i++) {
      var box = extent(layer, i);
      box = [
        Math.max(box[0], p[0]),
        Math.max(box[1], p[1]),
        Math.min(box[2], p[0] + p[2]),
        Math.min(box[3], p[1] + p[3]),
      ];
      if (!(box[0] <= box[2] && box[1] <= box[3])) {
        continue;
      }
      var c0 = Math.floor(box[0] / CELL);
      var c1 = Math.floor(box[2] / CELL);
      var r0 = alongX ? 0 : Math.floor(box[1] / CELL);
      var r1 = alongX ? 0 : Math.floor(box[3] / CELL);
      for (var c = c0; c <= c1; c++) {
        for (var r = r0; r <= r1; r++) {
          var key = c + "," + r;
          var bucket = cells.get(key);
          if (bucket === undefined) {
            cells.set(key, (bucket = []));
          }
          bucket.push(i);
        }
      }
    }
    layer.cells = cells;
    if (alongX && layer.targets.kind === "vertices") {
      // Lines: vertex indices of every line, sorted by x.
      var lines = new Map();
      var t = layer.targets;
      for (var j = 0; j < count(layer); j++) {
        var line = lines.get(t.line[j]);
        if (line === undefined) {
          lines.set(t.line[j], (line = []));
        }
        line.push(j);
      }
      lines.forEach(function (vertices) {
        vertices.sort(function (a, b) {
          return t.x[a] - t.x[b];
        });
      });
      layer.vertexLines = lines;
    }
  }

  function candidates(layer, x, y) {
    var key = Math.floor(x / CELL) + "," + (layer.lookup === "hover_x" ? 0 : Math.floor(y / CELL));
    return layer.cells.get(key) || [];
  }

  function insidePolygon(ring, x, y) {
    var inside = false;
    for (var i = 0, j = ring.length - 2; i + 1 < ring.length; j = i, i += 2) {
      var xi = ring[i], yi = ring[i + 1], xj = ring[j], yj = ring[j + 1];
      if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) {
        inside = !inside;
      }
    }
    return inside;
  }

  // Hits of one layer at (x, y): {distance, hits: [{target, row, x, y, radius}]}.
  function search(layer, x, y) {
    var t = layer.targets;
    var hits = [];
    var hit = function (i, hx, hy, radius) {
      return { target: i, row: t.rows[i], x: hx, y: hy, radius: radius || 0 };
    };
    if (layer.lookup === "hover_x" && layer.vertexLines) {
      // The vertex nearest to x on every line.
      var best = Infinity;
      layer.vertexLines.forEach(function (vertices) {
        var low = 0, high = vertices.length - 1;
        while (low < high) {
          var middle = (low + high) >> 1;
          if (t.x[vertices[middle]] < x) low = middle + 1;
          else high = middle;
        }
        var chosen = vertices[low];
        if (low > 0 && Math.abs(t.x[vertices[low - 1]] - x) <= Math.abs(t.x[chosen] - x)) {
          chosen = vertices[low - 1];
        }
        // Only lines that span the cursor's x (within a point's reach of an end).
        var first = t.x[vertices[0]], last = t.x[vertices[vertices.length - 1]];
        if (x < first - POINT_REACH || x > last + POINT_REACH) {
          return;
        }
        if (chosen !== undefined) {
          hits.push(hit(chosen, t.x[chosen], t.y[chosen]));
          best = Math.min(best, Math.abs(t.x[chosen] - x));
        }
      });
      // Keep the vertices aligned with the cursor (lets-plot `LocatedTargetsPicker`).
      hits = hits.filter(function (h) {
        return Math.abs(h.x - x) <= best;
      });
      return hits.length ? { distance: best, hits: hits } : null;
    }
    var list = candidates(layer, x, y);
    if (layer.lookup === "nearest") {
      var nearest = null, distance = Infinity;
      for (var k = 0; k < list.length; k++) {
        var i = list[k];
        var dx = t.x[i] - x, dy = t.y[i] - y;
        var d = Math.sqrt(dx * dx + dy * dy);
        var reach = (t.kind === "points" ? t.radius[i] : 0) + POINT_REACH;
        if (d <= reach && d < distance) {
          distance = d;
          nearest = i;
        }
      }
      if (nearest === null || distance > CUTOFF) {
        return null;
      }
      var radius = t.kind === "points" ? t.radius[nearest] : 0;
      return { distance: distance, hits: [hit(nearest, t.x[nearest], t.y[nearest], radius)] };
    }
    for (var m = 0; m < list.length; m++) {
      var n = list[m];
      if (t.kind === "rects") {
        var inX = x >= t.x[n] && x <= t.x[n] + t.width[n];
        var inY = y >= t.y[n] && y <= t.y[n] + t.height[n];
        if (inX && (inY || layer.lookup === "hover_x")) {
          // Bars point at their top; boxes and tiles at the cursor height.
          var top = layer.lookup === "hover_x" ? t.y[n] : y;
          hits.push(hit(n, t.x[n] + t.width[n] / 2, top));
        }
      } else if (t.kind === "polygons" && insidePolygon(t.rings[n], x, y)) {
        hits.push(hit(n, x, y));
      }
    }
    if (!hits.length) {
      return null;
    }
    if (layer.lookup === "hover") {
      hits = [hits[hits.length - 1]]; // the topmost shape
    }
    return { distance: AREA_DISTANCE, hits: hits };
  }

  function inPanel(layer, x, y) {
    var p = layer.panel;
    return x >= p[0] && x <= p[0] + p[2] && y >= p[1] && y <= p[1] + p[3];
  }

  // The closest layer result under the cursor (lets-plot picks one layer).
  function lookup(plot, x, y) {
    var best = null;
    plot.layers.forEach(function (layer) {
      if (!inPanel(layer, x, y)) {
        return;
      }
      var result = search(layer, x, y);
      if (result && (best === null || result.distance < best.distance)) {
        result.layer = layer;
        best = result;
      }
    });
    return best;
  }

  function element(tag, style, parent) {
    var node = document.createElement(tag);
    node.style.cssText = style;
    if (parent) {
      parent.appendChild(node);
    }
    return node;
  }

  // A general tooltip box for `row` of `layer`.
  function box(layer, row, overlay) {
    var root = element(
      "div",
      "position:absolute;left:0;top:0;box-sizing:border-box;display:flex;" +
        "background:#fff;border:1px solid " + TEXT + ";border-radius:4px;padding:6px;" +
        "color:" + TEXT + ";font:" + FONT + ";line-height:19px;white-space:nowrap;" +
        "pointer-events:none;",
      overlay
    );
    if (layer.min_width) {
      root.style.minWidth = layer.min_width + "px";
    }
    var marker = text(layer.marker, row);
    if (marker) {
      element(
        "div",
        "width:4px;flex:none;margin-right:6px;border-radius:1px;background:" + marker,
        root
      );
    }
    var body = element("div", "display:grid;grid-template-columns:auto auto;column-gap:8px;", root);
    var title = text(layer.title, row);
    if (title !== null) {
      var heading = element(
        "div",
        "grid-column:1/3;font-weight:bold;text-align:center;padding-bottom:3px;" +
          "margin-bottom:3px;border-bottom:0.7px solid " + TEXT + ";",
        body
      );
      heading.textContent = title;
    }
    var lines = 0;
    layer.lines.forEach(function (line) {
      var value = text(line, row);
      if (value === null) {
        return;
      }
      lines += 1;
      if (line.label) {
        element("div", "font-weight:bold;", body).textContent = line.label;
        element("div", "text-align:right;", body).textContent = value;
      } else {
        element("div", "grid-column:1/3;text-align:center;", body).textContent = value;
      }
    });
    if (lines === 0 && title === null) {
      overlay.removeChild(root);
      return null;
    }
    return root;
  }

  function axisBox(content, overlay) {
    var node = element(
      "div",
      "position:absolute;left:0;top:0;background:" + AXIS_FILL + ";color:#fff;font:" + FONT +
        ";line-height:15px;padding:3px 5px;white-space:nowrap;pointer-events:none;",
      overlay
    );
    node.textContent = content;
    return node;
  }

  function line(style, overlay) {
    return element("div", "position:absolute;pointer-events:none;background:#9e9e9e;" + style, overlay);
  }

  function clamp(value, low, high) {
    return Math.max(low, Math.min(value, high));
  }

  // Show the tooltips of `result` (scene pixels; `scale` converts to CSS pixels).
  function show(plot, result) {
    var overlay = plot.overlay;
    overlay.textContent = "";
    if (!result) {
      return;
    }
    var layer = result.layer;
    var scale = plot.scale;
    var width = plot.width * scale, height = plot.height * scale;
    var panel = layer.panel.map(function (v) {
      return v * scale;
    });
    var boxes = [];
    result.hits.forEach(function (h) {
      var node = box(layer, h.row, overlay);
      if (node) {
        boxes.push({ node: node, hit: h, w: node.offsetWidth, h: node.offsetHeight });
      }
    });
    var lead = result.hits[0];
    var anchor = layer.anchor;
    if (anchor && boxes.length) {
      // A fixed corner of the panel, plus crosshair lines through the target.
      var b = boxes[0];
      var parts = anchor.split("_");
      var left = parts[1] === "left" ? panel[0] + GAP
        : parts[1] === "right" ? panel[0] + panel[2] - b.w - GAP
        : panel[0] + (panel[2] - b.w) / 2;
      var top = parts[0] === "top" ? panel[1] + GAP
        : parts[0] === "bottom" ? panel[1] + panel[3] - b.h - GAP
        : panel[1] + (panel[3] - b.h) / 2;
      b.node.style.transform = "translate(" + left + "px," + top + "px)";
      line("left:" + panel[0] + "px;width:" + panel[2] + "px;top:" + lead.y * scale + "px;height:1px;", overlay);
      line("top:" + panel[1] + "px;height:" + panel[3] + "px;left:" + lead.x * scale + "px;width:1px;", overlay);
    } else if (boxes.length === 1) {
      // Above the target, else below; kept inside the plot.
      var one = boxes[0];
      var cx = one.hit.x * scale, cy = one.hit.y * scale, r = one.hit.radius * scale;
      var y = cy - r - OFFSET - one.h;
      if (y < 0) {
        y = cy + r + OFFSET;
      }
      var x = clamp(cx - one.w / 2, 0, Math.max(0, width - one.w));
      one.node.style.transform = "translate(" + x + "px," + clamp(y, 0, Math.max(0, height - one.h)) + "px)";
    } else if (boxes.length > 1) {
      // Beside the targets, stacked top to bottom without overlapping.
      boxes.sort(function (a, b) {
        return a.hit.y - b.hit.y;
      });
      var column = Math.max.apply(null, boxes.map(function (b) { return b.w; }));
      var cursor = lead.x * scale;
      var leftSide = cursor - OFFSET - column >= 0 || cursor + OFFSET + column > width;
      var next = 0;
      boxes.forEach(function (b) {
        b.top = Math.max(b.hit.y * scale - b.h / 2, next);
        next = b.top + b.h + GAP;
      });
      var overflow = next - GAP - height;
      boxes.forEach(function (b) {
        var top = overflow > 0 ? Math.max(0, b.top - overflow) : b.top;
        var x = leftSide ? Math.max(0, cursor - OFFSET - b.w) : cursor + OFFSET;
        b.node.style.transform = "translate(" + x + "px," + top + "px)";
      });
    }
    // Axis tooltips at the panel edges.
    var axisX = text(layer.axis_x, lead.row);
    if (axisX !== null) {
      var ax = axisBox(axisX, overlay);
      var axLeft = clamp(lead.x * scale - ax.offsetWidth / 2, 0, Math.max(0, width - ax.offsetWidth));
      ax.style.transform = "translate(" + axLeft + "px," + (panel[1] + panel[3] + 2) + "px)";
    }
    var axisY = text(layer.axis_y, lead.row);
    if (axisY !== null) {
      var ay = axisBox(axisY, overlay);
      var ayTop = clamp(lead.y * scale - ay.offsetHeight / 2, 0, Math.max(0, height - ay.offsetHeight));
      ay.style.transform = "translate(" + Math.max(0, panel[0] - ay.offsetWidth - 2) + "px," + ayTop + "px)";
    }
  }

  function attach(plot) {
    var data = plot.root.querySelector("script.ggrust-tooltips");
    if (!data || !plot.svg) {
      return;
    }
    var payload;
    try {
      payload = JSON.parse(data.textContent);
    } catch (error) {
      return;
    }
    plot.layers = payload.layers || [];
    plot.layers.forEach(index);
    var viewBox = plot.svg.viewBox && plot.svg.viewBox.baseVal;
    plot.width = viewBox && viewBox.width ? viewBox.width : plot.svg.width.baseVal.value;
    plot.height = viewBox && viewBox.height ? viewBox.height : plot.svg.height.baseVal.value;
    plot.overlay = element("div", "position:absolute;left:0;top:0;width:0;height:0;line-height:normal;z-index:1;", plot.root);
    var pending = null;
    var move = function (event) {
      pending = event;
      if (plot.frame) {
        return;
      }
      plot.frame = window.requestAnimationFrame(function () {
        plot.frame = 0;
        var bounds = plot.svg.getBoundingClientRect();
        if (!bounds.width) {
          return;
        }
        // The SVG may be scaled by the page's CSS: scene pixels per CSS pixel.
        plot.scale = bounds.width / plot.width;
        var x = (pending.clientX - bounds.left) / plot.scale;
        var y = (pending.clientY - bounds.top) / plot.scale;
        show(plot, lookup(plot, x, y));
      });
    };
    plot.svg.addEventListener("mousemove", move);
    plot.svg.addEventListener("mouseleave", function () {
      if (plot.frame) {
        window.cancelAnimationFrame(plot.frame);
        plot.frame = 0;
      }
      show(plot, null);
    });
  }

  // Attach to the plot whose root element has id `id`. Mounting twice returns
  // the same object. The plot lives on its root element, so removing the output
  // from the page frees it (no page-wide registry).
  function mount(id) {
    var root = document.getElementById(id);
    if (root === null) {
      return null;
    }
    if (root.ggrust) {
      return root.ggrust;
    }
    var plot = { id: id, root: root, svg: root.querySelector("svg"), layers: [] };
    root.ggrust = plot;
    root.setAttribute("data-ggrust-mounted", version);
    attach(plot);
    return plot;
  }

  window.ggrust = { version: version, mount: mount, lookup: lookup };
})
