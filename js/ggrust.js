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
// With `ggtb()` it also adds lets-plot's toolbar (pan, rubber band zoom,
// centerpoint zoom, reset) and zooms panels: see "Zoom and pan" below.
// What it does not do (yet): canvas point layers (plan.md 4.4).
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

  // ---- Zoom and pan (`ggtb()`; lets-plot's toolbar and tools) ----
  //
  // The engine writes, for every panel, both screen axes' domains, their break
  // rules and templates of their grid lines, ticks and labels drawn at pixel 0
  // (crates/ggrust-core/src/layout/zoom.rs). A zoomed panel maps each mark
  // through its new domain: lines, areas, bars and rasters are scaled (strokes
  // keep their width), points and texts move with their anchor and keep their
  // size (points grow with `size_zoomin`). Axes and grid are redrawn from the
  // templates at breaks computed here: the engine's linear breaks and default
  // number format (crates/ggrust-core/src/scale/breaks.rs, format.rs), ported.

  var SVG_NS = "http://www.w3.org/2000/svg";
  var CLOSE_G = "<" + "/g>";
  // lets-plot DefaultToolbarJs colors; the background is C_BACKGR with alpha 0.8.
  var BAR_BACKGROUND = "rgba(245, 246, 248, 0.8)";
  var BUTTON_STROKE = "rgb(110, 110, 110)";
  var BUTTON_HOVER = "rgb(218, 219, 221)";
  var BUTTON_SELECTED = "rgb(69, 114, 232)";
  var MIN_SELECTION = 15; // px (DrawRectFeedback.MIN_SIZE)
  var WHEEL_STEP = 0.08; // WheelZoomFeedback: one wheel notch
  var MIN_EXPONENT = -7, MAX_EXPONENT = 6; // number format: decimal notation range
  var MAX_DECIMAL_PLACES = 12;
  var MIN_POSITIVE = 4.9e-324 * 1e5, MAX_VALUE = Number.MAX_VALUE / 1e5;

  function icon(paths) {
    return '<svg xmlns="' + SVG_NS + '" width="16" height="16" viewBox="0 0 16 16"><g stroke="none">' +
      paths + CLOSE_G + "<" + "/svg>";
  }
  var FRAME = '<path d="M3,2C1.901,2 1,2.901 1,4v8c0,1.099 0.901,2 2,2h10c1.099,0 2,-0.901 2,-2V4c0,-1.099 -0.901,-2 -2,-2ZM3,3h10c0.563,0 1,0.438 1,1v8c0,0.563 -0.438,1 -1,1H3c-0.563,0 -1,-0.438 -1,-1V4c0,-0.563 0.438,-1 1,-1Z"/>';
  var TOOLS = [
    { name: "pan", label: "Pan", icon: icon('<path d="M7.5,1V6.793L4.354,3.646 3.646,4.354 6.793,7.5H1v1h5.793L3.646,11.646 4.354,12.354 7.5,9.207V15h1V9.207L11.646,12.354 12.354,11.646 9.207,8.5H15v-1H9.207L12.354,4.354 11.646,3.646 8.5,6.793V1Z"/>') },
    { name: "corner", label: "Rubber Band Zoom", icon: icon('<circle cx="1.6" cy="2.6" r="1.5"/>' + FRAME) },
    { name: "center", label: "Centerpoint Zoom", icon: icon('<circle cx="8" cy="8" r="1.5"/>' + FRAME) },
  ];
  var RESET_ICON = icon('<path d="M8.322,1.482C6.095,1.34 3.77,2.313 2.367,4.75l0.865,0.5C4.867,2.41 7.757,1.939 10.098,2.908 12.439,3.877 14.152,6.255 13.311,9.41 12.469,12.566 9.8,13.773 7.287,13.447 4.775,13.122 2.504,11.276 2.5,8h-1c0.004,3.75 2.727,6.06 5.658,6.439C10.09,14.819 13.314,13.282 14.277,9.668 15.241,6.054 13.212,3.115 10.48,1.984 9.798,1.702 9.065,1.53 8.322,1.482Z"/><path d="M2,1v4.5h4.5v-1H3V1Z"/>');

  // -- Breaks (crates/ggrust-core/src/scale/breaks.rs) --

  function beyondPrecision(base, delta) {
    return base !== 0 && Math.log10(Math.abs(base)) - Math.log10(Math.abs(delta)) > MAX_DECIMAL_PLACES;
  }

  function roundAway(value) {
    return value < 0 ? -Math.round(-value) : Math.round(value);
  }

  function roundToDecimals(value, decimals) {
    if (!isFinite(value) || Math.abs(decimals) > 22) {
      return value;
    }
    var rounded;
    if (decimals >= 0) {
      var scale = Math.pow(10, decimals);
      rounded = roundAway(value * scale) / scale;
    } else {
      var unit = Math.pow(10, -decimals);
      rounded = roundAway(value / unit) * unit;
    }
    return isFinite(rounded) ? rounded : value;
  }

  function niceStep(span, count) {
    var step = Math.pow(10, Math.floor(Math.log10(span / count)));
    var error = (step * count) / span;
    if (error <= 0.15) return step * 10;
    if (error <= 0.35) return step * 5;
    if (error <= 0.75) return step * 2;
    return step;
  }

  function breaksWithStep(low, high, step) {
    var tolerance = step / 10000;
    var first = Math.ceil((low - tolerance) / step);
    var last = Math.floor((high + tolerance) / step);
    if (!isFinite(first) || !isFinite(last) || last - first > 100000) {
      return [];
    }
    var decimals = 1 - Math.floor(Math.log10(step));
    var breaks = [];
    for (var multiple = first; multiple <= last; multiple += 1) {
      var value = Math.min(Math.max(roundToDecimals(multiple * step, decimals) + 0, low), high);
      if (breaks[breaks.length - 1] !== value) {
        breaks.push(value);
      }
    }
    return breaks;
  }

  function linearBreaks(low, high, count) {
    if (low > high) {
      var swap = low; low = high; high = swap;
    }
    if (!isFinite(low) || !isFinite(high) || count === 0) {
      return [];
    }
    var step = niceStep(high - low, count);
    if (!(isFinite(step) && step > 0) || beyondPrecision(low, step) || beyondPrecision(high, step)) {
      return [low];
    }
    var breaks = breaksWithStep(low, high, step);
    return breaks.length ? breaks : [low];
  }

  // -- Transforms (crates/ggrust-core/src/scale/transform.rs) --

  function logLimits(base) {
    return [Math.log(MIN_POSITIVE) / Math.log(base), Math.log(MAX_VALUE) / Math.log(base)];
  }

  function inDomain(transform, value) {
    if (!isFinite(value)) return false;
    if (transform === "sqrt") return value >= 0;
    if (transform === "log10" || transform === "log2") return value >= MIN_POSITIVE && value <= MAX_VALUE;
    return true;
  }

  function applyTransform(transform, value) {
    if (!inDomain(transform, value)) return null;
    switch (transform) {
      case "reverse": return -value;
      case "sqrt": return Math.sqrt(value);
      case "log10":
      case "log2": {
        var base = transform === "log10" ? 10 : 2;
        var limits = logLimits(base);
        var log = Math.log(value) / Math.log(base);
        return isNaN(log) ? log : clamp(log, limits[0], limits[1]);
      }
      default: return value;
    }
  }

  function inverseTransform(transform, value) {
    switch (transform) {
      case "reverse": return -value;
      case "sqrt": return value * value;
      case "log10":
      case "log2": {
        var base = transform === "log10" ? 10 : 2;
        var limits = logLimits(base);
        return Math.pow(base, clamp(value, limits[0], limits[1]));
      }
      default: return value;
    }
  }

  // -- Labels (crates/ggrust-core/src/scale/format.rs) --

  function precisionOf(reference, step) {
    if (reference === 0) reference = 2.2250738585072014e-308 * 10;
    if (step === 0) step = reference / 10;
    var domainPower = Math.log10(reference), stepPower = Math.log10(step);
    var scientific = (domainPower < 0 && stepPower <= MIN_EXPONENT) ||
      (domainPower >= MAX_EXPONENT && stepPower > 2);
    var precision = scientific ? domainPower - stepPower + 1 :
      stepPower > 0 ? Math.ceil(domainPower) : Math.ceil(domainPower) - stepPower;
    return Math.ceil(precision - 0.001);
  }

  // The digits of |value| rounded half up to 1 + `fraction` digits, and the exponent.
  function roundedDigits(value, fraction) {
    var shortest = Math.abs(value).toExponential().split("e");
    var exponent = parseInt(shortest[1], 10);
    var digits = shortest[0].replace(".", "").split("").map(Number);
    var keep = 1 + fraction;
    if (digits.length > keep) {
      var up = digits[keep] >= 5;
      digits.length = keep;
      if (up) {
        var index = keep;
        for (;;) {
          if (index === 0) {
            digits.unshift(1);
            digits.pop();
            exponent += 1;
            break;
          }
          index -= 1;
          if (digits[index] === 9) {
            digits[index] = 0;
          } else {
            digits[index] += 1;
            break;
          }
        }
      }
    }
    return { digits: digits.join(""), exponent: exponent };
  }

  function groupThousands(integer) {
    var grouped = "";
    for (var i = 0; i < integer.length; i++) {
      if (i > 0 && (integer.length - i) % 3 === 0) grouped += ",";
      grouped += integer[i];
    }
    return grouped;
  }

  function trimFraction(mantissa) {
    var parts = mantissa.split(".");
    if (parts.length < 2) return mantissa;
    var fraction = parts[1].replace(/0+$/, "");
    return fraction ? parts[0] + "." + fraction : parts[0];
  }

  // `g` formatting with `precision` significant digits: {text, superscript}.
  function formatGeneral(value, precision) {
    if (!isFinite(value)) {
      return { text: value !== value ? "NaN" : value > 0 ? "inf" : "-inf", superscript: null };
    }
    var rounded = roundedDigits(value, Math.max(precision - 1, 0));
    var digits = rounded.digits, exponent = rounded.exponent;
    if (/^0*$/.test(digits)) {
      return { text: "0", superscript: null };
    }
    var sign = value < 0 ? "-" : "";
    if (exponent > MIN_EXPONENT && exponent < MAX_EXPONENT) {
      var point = exponent + 1, integer, fraction;
      if (point <= 0) {
        integer = "0";
        fraction = new Array(1 - point).join("0") + digits;
      } else if (point >= digits.length) {
        integer = digits + new Array(point - digits.length + 1).join("0");
        fraction = "";
      } else {
        integer = digits.slice(0, point);
        fraction = digits.slice(point);
      }
      fraction = fraction.replace(/0+$/, "");
      integer = groupThousands(integer);
      return { text: sign + (fraction ? integer + "." + fraction : integer), superscript: null };
    }
    var mantissa = trimFraction(digits.length > 1 ? digits[0] + "." + digits.slice(1) : digits);
    return {
      text: mantissa === "1" ? sign + "10" : sign + mantissa + "·10",
      superscript: String(exponent).replace("-", "−"),
    };
  }

  function formatBreaks(values) {
    var reference, step;
    if (values.length === 0) {
      reference = 0; step = 0.5;
    } else if (values.length === 1) {
      reference = Math.abs(values[0]); step = reference / 10;
    } else {
      reference = Math.max(Math.abs(values[0]), Math.abs(values[values.length - 1]));
      step = Math.abs(values[1] - values[0]);
    }
    var precision = precisionOf(reference, step);
    return values.map(function (value) { return formatGeneral(value, precision); });
  }

  function formatIndividually(values) {
    return values.map(function (value, index) {
      var step = values.length === 1 ? value / 10 : index === 0 ? value - values[1] : value - values[index - 1];
      return formatGeneral(value, precisionOf(Math.abs(value), Math.abs(step)));
    });
  }

  // Generated breaks of an axis over [low, high] (transformed units), as
  // `ContinuousPositionScale::generated_breaks`: {positions, labels}.
  function generatedBreaks(transform, low, high, count) {
    var a = inverseTransform(transform, low), b = inverseTransform(transform, high);
    var dataLow = Math.min(a, b), dataHigh = Math.max(a, b);
    var values;
    if (transform === "identity" || transform === "reverse") {
      values = linearBreaks(dataLow, dataHigh, count);
      return {
        positions: values.map(function (v) { return inverseTransform(transform, v); }),
        labels: formatBreaks(values),
      };
    }
    var tLow = applyTransform(transform, dataLow), tHigh = applyTransform(transform, dataHigh);
    if (tLow === null || tHigh === null) {
      return { positions: [], labels: [] };
    }
    var lowT = Math.min(tLow, tHigh), highT = Math.max(tLow, tHigh);
    if (transform === "log10") {
      var powers = Math.round(Math.floor(highT) - Math.ceil(lowT)) + 1;
      if (powers >= 3 && powers <= count) {
        count = powers;
      }
    }
    values = linearBreaks(lowT, highT, count)
      .map(function (p) { return inverseTransform(transform, p); })
      .filter(function (v) { return inDomain(transform, v); });
    return {
      positions: values.map(function (v) {
        var p = applyTransform(transform, v);
        return p === null ? NaN : p;
      }),
      labels: formatIndividually(values),
    };
  }

  // lets-plot `getFlexBreaks`: at most `count` breaks.
  function flexBreaks(transform, low, high, count) {
    var breaks = generatedBreaks(transform, low, high, count);
    if (count === 1 && breaks.positions.length) {
      return { positions: breaks.positions.slice(0, 1), labels: breaks.labels.slice(0, 1) };
    }
    var target = count;
    while (breaks.positions.length > count) {
      var delta = Math.max(Math.floor((breaks.positions.length - count) / 2), 1);
      if (target <= delta + 1) {
        return generatedBreaks(transform, low, high, 1);
      }
      target -= delta;
      breaks = generatedBreaks(transform, low, high, target);
    }
    return breaks;
  }

  // -- Drawing a zoomed panel --

  function escapeXml(text) {
    return String(text).split("&").join("&amp;").split("<").join("&lt;").split(">").join("&gt;");
  }

  // The pixel of `position` on `axis` showing `view`.
  function pixelOf(axis, view, position) {
    var p = axis.pixels;
    return p[0] + ((position - view[0]) / (view[1] - view[0])) * (p[1] - p[0]);
  }

  // The position at `pixel` on `axis` showing `view`.
  function positionAt(axis, view, pixel) {
    var p = axis.pixels;
    return view[0] + ((pixel - p[0]) / (p[1] - p[0])) * (view[1] - view[0]);
  }

  // Unzoomed pixel -> zoomed pixel on one axis: {scale, offset}.
  function affine(axis, view) {
    var d = axis.domain, p = axis.pixels;
    var scale = (d[1] - d[0]) / (view[1] - view[0]);
    return { scale: scale, offset: pixelOf(axis, view, d[0]) - scale * p[0] };
  }

  // Breaks of `axis` inside `view`: [{position, pixel, svg}] with the label's
  // SVG translated along the axis by `along(pixel delta)`.
  function visibleBreaks(axis, view) {
    var low = Math.min(view[0], view[1]), high = Math.max(view[0], view[1]);
    var tolerance = (high - low) * 1e-9;
    var result = [];
    if (axis.fixed) {
      axis.fixed.forEach(function (item) {
        if (item[0] >= low - tolerance && item[0] <= high + tolerance) {
          var pixel = pixelOf(axis, view, item[0]);
          result.push({ position: item[0], pixel: pixel, label: item[2], shift: pixel - item[1] });
        }
      });
      return result;
    }
    var breaks = flexBreaks(axis.transform, low, high, Math.max(axis.count, 1));
    breaks.positions.forEach(function (position, index) {
      var label = breaks.labels[index];
      var template = label.superscript === null ? axis.label : axis.label_superscript;
      // Function replacements: a `$` in a label is literal.
      var svg = template && template
        .replace(">@<", function () { return ">" + escapeXml(label.text) + "<"; })
        .replace(">^<", function () { return ">" + escapeXml(label.superscript || "") + "<"; });
      result.push({ position: position, pixel: pixelOf(axis, view, position), label: svg, shift: pixelOf(axis, view, position) });
    });
    return result;
  }

  function translated(svg, horizontal, shift) {
    return '<g transform="translate(' + (horizontal ? shift + " 0" : "0 " + shift) + ')">' + svg + CLOSE_G;
  }

  // Grid and axis contents of a zoomed tile, as SVG strings.
  function axisContent(tile) {
    var spec = tile.spec, views = tile.views;
    var h = visibleBreaks(spec.horizontal, views.h), v = visibleBreaks(spec.vertical, views.v);
    var panel = spec.panel;
    var minor = function (axis, view, breaks, horizontal, out) {
      if (!axis.minor || breaks.length < 2) return;
      var step = breaks[1].position - breaks[0].position;
      var start = breaks[0].position - step / 2;
      var from = horizontal ? panel[0] : panel[1], to = from + (horizontal ? panel[2] : panel[3]);
      for (var i = 0; i <= breaks.length; i++) {
        var pixel = pixelOf(axis, view, start + i * step);
        if (pixel >= from && pixel <= to) out.push(translated(axis.minor, horizontal, pixel));
      }
    };
    var grid = [];
    minor(spec.horizontal, views.h, h, true, grid);
    minor(spec.vertical, views.v, v, false, grid);
    [[spec.horizontal, h, true], [spec.vertical, v, false]].forEach(function (entry) {
      if (entry[0].major) {
        entry[1].forEach(function (b) { grid.push(translated(entry[0].major, entry[2], b.pixel)); });
      }
    });
    var axisNodes = function (axis, breaks, horizontal) {
      var out = axis.line ? [axis.line] : [];
      breaks.forEach(function (b) {
        if (axis.tick) out.push(translated(axis.tick, horizontal, b.pixel));
        if (b.label) out.push(translated(b.label, horizontal, b.shift));
      });
      return out.join("");
    };
    return { grid: grid.join(""), x: axisNodes(spec.horizontal, h, true), y: axisNodes(spec.vertical, v, false) };
  }

  function childByClass(parent, name) {
    for (var child = parent.firstElementChild; child; child = child.nextElementSibling) {
      if (child.getAttribute("class") === name) return child;
    }
    return null;
  }

  // The marks of a panel group: circles move by their centre, anchored groups
  // with their anchor, everything else is scaled.
  function collectMarks(container, marks) {
    for (var child = container.firstElementChild; child; child = child.nextElementSibling) {
      if (child.getAttribute("class") === "ggrust-points") {
        for (var m = child.firstElementChild; m; m = m.nextElementSibling) {
          if (m.tagName.toLowerCase() === "circle") {
            marks.push({ kind: "circle", element: m, x: +m.getAttribute("cx"), y: +m.getAttribute("cy"), r: +m.getAttribute("r") });
          } else {
            marks.push(anchored(m, "marker"));
          }
        }
      } else if (child.hasAttribute("data-anchor")) {
        marks.push(anchored(child, "anchor"));
      } else if (child.tagName.toLowerCase() === "text") {
        // A text outside an anchored group moves with its own position.
        marks.push({ kind: "anchor", element: child, x: +child.getAttribute("x"), y: +child.getAttribute("y") });
      } else if (child.tagName.toLowerCase() === "g" && child.querySelector(".ggrust-points,[data-anchor]")) {
        collectMarks(child, marks);
      } else {
        marks.push({ kind: "shape", element: child });
      }
    }
  }

  function anchored(element, kind) {
    var anchor = (element.getAttribute("data-anchor") || "0 0").split(" ").map(Number);
    return { kind: kind, element: element, x: anchor[0], y: anchor[1] };
  }

  function sizeFactor(zoom, tile) {
    var spec = tile.spec;
    var span = function (axis, view) { return (axis.domain[1] - axis.domain[0]) / (view[1] - view[0]); };
    var x = span(spec.horizontal, tile.views.h), y = span(spec.vertical, tile.views.v);
    var factor = { x: x, y: y, min: Math.min(x, y) }[zoom.payload.size_basis];
    if (factor === undefined) factor = Math.max(x, y);
    var limit = zoom.payload.size_zoomin === 0 ? 1 : zoom.payload.size_zoomin === -1 ? Infinity : zoom.payload.size_zoomin;
    return clamp(factor, 1, Math.max(limit, 1));
  }

  function isUnzoomed(tile) {
    var same = function (a, b) { return a[0] === b[0] && a[1] === b[1]; };
    return same(tile.views.h, tile.spec.horizontal.domain) && same(tile.views.v, tile.spec.vertical.domain);
  }

  // Move every mark, redraw grid and axes, and move the tooltip targets of `tile`.
  function drawTile(plot, tile) {
    var element = tile.element;
    var panelGroup = childByClass(element, "panel");
    if (!tile.saved) {
      var grid = childByClass(element, "grid");
      if (!grid && panelGroup) {
        grid = document.createElementNS(SVG_NS, "g");
        grid.setAttribute("class", "grid");
        element.insertBefore(grid, panelGroup);
      }
      tile.groups = { grid: grid, x: childByClass(element, "axis-x"), y: childByClass(element, "axis-y") };
      tile.saved = {};
      Object.keys(tile.groups).forEach(function (key) {
        if (tile.groups[key]) tile.saved[key] = tile.groups[key].innerHTML;
      });
      tile.marks = [];
      if (panelGroup) collectMarks(panelGroup, tile.marks);
    }
    var reset = isUnzoomed(tile);
    var h = affine(tile.spec.horizontal, tile.views.h), v = affine(tile.spec.vertical, tile.views.v);
    var k = reset ? 1 : sizeFactor(plot.zoom, tile);
    var matrix = "matrix(" + h.scale + " 0 0 " + v.scale + " " + h.offset + " " + v.offset + ")";
    tile.marks.forEach(function (mark) {
      var e = mark.element;
      if (mark.kind === "shape") {
        if (reset) e.removeAttribute("transform");
        else e.setAttribute("transform", matrix);
        return;
      }
      var x = h.scale * mark.x + h.offset, y = v.scale * mark.y + v.offset;
      if (mark.kind === "circle") {
        e.setAttribute("cx", x);
        e.setAttribute("cy", y);
        e.setAttribute("r", mark.r * k);
      } else if (reset) {
        e.removeAttribute("transform");
      } else if (mark.kind === "marker" && k !== 1) {
        e.setAttribute("transform", "translate(" + x + " " + y + ") scale(" + k + ") translate(" + -mark.x + " " + -mark.y + ")");
      } else {
        e.setAttribute("transform", "translate(" + (x - mark.x) + " " + (y - mark.y) + ")");
      }
    });
    var content = reset ? tile.saved : axisContent(tile);
    Object.keys(tile.groups).forEach(function (key) {
      if (tile.groups[key]) tile.groups[key].innerHTML = content[key] || "";
    });
    moveTargets(plot, tile, h, v, k, reset);
  }

  function samePanel(a, b) {
    for (var i = 0; i < 4; i++) {
      if (Math.abs(a[i] - b[i]) > 0.01) return false;
    }
    return true;
  }

  // Tooltip targets of the layers in `tile`'s panel, through the same mapping.
  function moveTargets(plot, tile, h, v, k, reset) {
    (plot.layers || []).forEach(function (layer) {
      if (!samePanel(layer.panel, tile.spec.panel)) return;
      if (!layer.original) layer.original = layer.targets;
      var o = layer.original;
      if (reset) {
        layer.targets = o;
        index(layer);
        return;
      }
      var mx = function (value) { return h.scale * value + h.offset; };
      var my = function (value) { return v.scale * value + v.offset; };
      var t = {};
      Object.keys(o).forEach(function (key) { t[key] = o[key]; });
      if (o.kind === "polygons") {
        t.rings = o.rings.map(function (ring) {
          return ring.map(function (value, i) { return i % 2 ? my(value) : mx(value); });
        });
      } else {
        t.x = o.x.map(mx);
        t.y = o.y.map(my);
        if (o.kind === "points") t.radius = o.radius.map(function (r) { return r * k; });
        if (o.kind === "rects") {
          t.width = o.width.map(function (w) { return w * h.scale; });
          t.height = o.height.map(function (value) { return value * v.scale; });
        }
      }
      layer.targets = t;
      index(layer);
    });
  }

  // -- Views --

  // Set the views of `tile` (and of the tiles sharing an axis with it) from a
  // viewport in its pixels, `[left, top, right, bottom]`; `null` keeps an axis.
  function applyViewport(plot, ref, start, hRange, vRange) {
    var zoomPlot = plot.zoom.plots[ref.plot];
    var tile = zoomPlot.tiles[ref.tile];
    var hView = hRange && order([
      positionAt(tile.spec.horizontal, start.h, hRange[0]),
      positionAt(tile.spec.horizontal, start.h, hRange[1]),
    ]);
    var vView = vRange && order([
      positionAt(tile.spec.vertical, start.v, vRange[0]),
      positionAt(tile.spec.vertical, start.v, vRange[1]),
    ]);
    zoomPlot.tiles.forEach(function (other, index) {
      var self = index === ref.tile;
      if (hView && (self || zoomPlot.shared[0])) other.views.h = hView;
      if (vView && (self || zoomPlot.shared[1])) other.views.v = vView;
      if (self || zoomPlot.shared[0] || zoomPlot.shared[1]) other.dirty = true;
    });
    schedule(plot);
  }

  function order(view) {
    return view[0] <= view[1] ? view : [view[1], view[0]];
  }

  function schedule(plot) {
    if (plot.zoom.frame) return;
    plot.zoom.frame = window.requestAnimationFrame(function () {
      plot.zoom.frame = 0;
      plot.zoom.plots.forEach(function (zoomPlot) {
        zoomPlot.tiles.forEach(function (tile) {
          if (tile.dirty) {
            tile.dirty = false;
            drawTile(plot, tile);
          }
        });
      });
    });
  }

  function resetViews(plot) {
    plot.zoom.plots.forEach(function (zoomPlot) {
      zoomPlot.tiles.forEach(function (tile) {
        tile.views = { h: tile.spec.horizontal.domain.slice(), v: tile.spec.vertical.domain.slice() };
        tile.dirty = true;
      });
    });
    schedule(plot);
  }

  // -- Pointer interactions --

  function scenePoint(plot, event) {
    var bounds = plot.svg.getBoundingClientRect();
    var scale = bounds.width / plot.width;
    return [(event.clientX - bounds.left) / scale, (event.clientY - bounds.top) / scale];
  }

  function tileAt(plot, point) {
    var found = null;
    plot.zoom.plots.forEach(function (zoomPlot, p) {
      zoomPlot.tiles.forEach(function (tile, t) {
        var r = tile.spec.panel;
        if (point[0] >= r[0] && point[0] <= r[0] + r[2] && point[1] >= r[1] && point[1] <= r[1] + r[3]) {
          found = { plot: p, tile: t };
        }
      });
    });
    return found;
  }

  function tileOf(plot, ref) {
    return plot.zoom.plots[ref.plot].tiles[ref.tile];
  }

  function currentViews(tile) {
    return { h: tile.views.h.slice(), v: tile.views.v.slice() };
  }

  // The selection of a box zoom (DrawRectFeedback selectors): {rect: [l, t, r, b],
  // acceptable (large enough), only: the one axis a band zooms}. Bands span the
  // panel: a horizontal band zooms the vertical axis, a vertical band the other.
  function selection(drag, to, centre) {
    var r = drag.bounds, from = drag.from;
    var limit = function (p) {
      return [clamp(p[0], r[0], r[0] + r[2]), clamp(p[1], r[1], r[1] + r[3])];
    };
    var dx = to[0] - from[0], dy = to[1] - from[1];
    if (!drag.mode) {
      if (Math.sqrt(dx * dx + dy * dy) > 20) {
        drag.mode = Math.abs(dx) < 7 ? "horizontal" : Math.abs(dy) < 7 ? "vertical" : "box";
      } else {
        return { rect: span(from, to), acceptable: false };
      }
    }
    var rect;
    if (drag.mode === "horizontal") {
      if (centre) {
        rect = [r[0], from[1] - Math.abs(dy), r[0] + r[2], from[1] + Math.abs(dy)];
      } else {
        var y = limit(to)[1];
        rect = [r[0], Math.min(from[1], y), r[0] + r[2], Math.max(from[1], y)];
      }
      return { rect: rect, acceptable: rect[3] - rect[1] > MIN_SELECTION, only: "vertical" };
    }
    if (drag.mode === "vertical") {
      if (centre) {
        rect = [from[0] - Math.abs(dx), r[1], from[0] + Math.abs(dx), r[1] + r[3]];
      } else {
        var x = limit(to)[0];
        rect = [Math.min(from[0], x), r[1], Math.max(from[0], x), r[1] + r[3]];
      }
      return { rect: rect, acceptable: rect[2] - rect[0] > MIN_SELECTION, only: "horizontal" };
    }
    if (centre) {
      var ratio = r[2] / r[3];
      var w = ratio > 1 ? Math.abs(dy) * ratio : Math.abs(dx);
      var hh = w / ratio;
      rect = [from[0] - w, from[1] - hh, from[0] + w, from[1] + hh];
    } else {
      rect = span(from, limit(to));
    }
    return { rect: rect, acceptable: rect[2] - rect[0] > MIN_SELECTION || rect[3] - rect[1] > MIN_SELECTION };
  }

  function span(a, b) {
    return [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.max(a[0], b[0]), Math.max(a[1], b[1])];
  }

  // The dashed selection rectangle and the grey shade around it.
  function drawSelection(plot, drag, result) {
    var r = drag.bounds, s = result.rect;
    var layer = plot.zoom.decorations;
    var shade = result.acceptable
      ? '<path fill="#c0c0c0" fill-opacity="0.5" fill-rule="evenodd" d="M' + r[0] + " " + r[1] + "H" + (r[0] + r[2]) +
        "V" + (r[1] + r[3]) + "H" + r[0] + "ZM" + s[0] + " " + s[1] + "H" + s[2] + "V" + s[3] + "H" + s[0] + 'Z"/>'
      : "";
    layer.innerHTML = shade + '<rect fill="none" stroke="#000000" stroke-width="1.5" stroke-dasharray="5,5" x="' +
      s[0] + '" y="' + s[1] + '" width="' + (s[2] - s[0]) + '" height="' + (s[3] - s[1]) + '"/>';
  }

  function onPress(plot, event) {
    var zoom = plot.zoom;
    if (!zoom.tool || event.button !== 0) return;
    var from = scenePoint(plot, event);
    var ref = tileAt(plot, from);
    if (!ref) return;
    event.preventDefault();
    var tile = tileOf(plot, ref);
    zoom.drag = { ref: ref, from: from, start: currentViews(tile), bounds: tile.spec.panel, mode: null };
    plot.dragging = true;
    if (plot.overlay) show(plot, null);
    var move = function (e) { onDrag(plot, e); };
    var up = function (e) {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
      onRelease(plot, e);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  }

  function onDrag(plot, event) {
    var zoom = plot.zoom, drag = zoom.drag;
    if (!drag) return;
    var to = scenePoint(plot, event);
    var r = drag.bounds;
    if (zoom.tool === "pan") {
      var dx = to[0] - drag.from[0], dy = to[1] - drag.from[1];
      if (!drag.mode) {
        if (Math.sqrt(dx * dx + dy * dy) <= 15) return;
        drag.mode = Math.abs(dx) < 7 ? "vertical" : Math.abs(dy) < 7 ? "horizontal" : "free";
      }
      var hRange = drag.mode === "vertical" ? null : [r[0] - dx, r[0] + r[2] - dx];
      var vRange = drag.mode === "horizontal" ? null : [r[1] - dy, r[1] + r[3] - dy];
      applyViewport(plot, drag.ref, drag.start, hRange, vRange);
      return;
    }
    drawSelection(plot, drag, selection(drag, to, zoom.tool === "center"));
  }

  function onRelease(plot, event) {
    var zoom = plot.zoom, drag = zoom.drag;
    zoom.drag = null;
    plot.dragging = false;
    zoom.decorations.innerHTML = "";
    if (!drag || zoom.tool === "pan") return;
    var result = selection(drag, scenePoint(plot, event), zoom.tool === "center");
    if (!result.acceptable) return;
    var s = result.rect;
    applyViewport(plot, drag.ref, drag.start,
      result.only === "vertical" ? null : [s[0], s[2]],
      result.only === "horizontal" ? null : [s[3], s[1]]);
  }

  function onWheel(plot, event) {
    var zoom = plot.zoom;
    if (!zoom.tool || zoom.drag) return;
    var origin = scenePoint(plot, event);
    var ref = tileAt(plot, origin);
    if (!ref) return;
    event.preventDefault();
    // Normalized like lets-plot's scroll amount: about 1 per wheel notch.
    var amount = event.deltaY / (event.deltaMode === 1 ? 3 : event.deltaMode === 2 ? 1 : 100);
    if (!amount) return;
    var step = Math.abs(amount) > 0.3 ? WHEEL_STEP : Math.abs(amount) / 10;
    var factor = amount < 0 ? 1 - step : 1 + step;
    var tile = tileOf(plot, ref), r = tile.spec.panel;
    var scaled = function (low, size, centre) {
      return [low + (centre - low) * (1 - factor), low + (centre - low) * (1 - factor) + size * factor];
    };
    var hRange = scaled(r[0], r[2], origin[0]);
    var vRange = scaled(r[1], r[3], origin[1]);
    applyViewport(plot, ref, currentViews(tile), hRange, vRange);
  }

  // -- Toolbar --

  function setButton(button, selected) {
    button.style.backgroundColor = selected ? BUTTON_SELECTED : "transparent";
    button.setAttribute("data-selected", selected ? "1" : "");
    var svg = button.querySelector("svg");
    if (svg) {
      svg.style.stroke = selected ? "white" : BUTTON_STROKE;
      svg.style.fill = selected ? "white" : BUTTON_STROKE;
    }
  }

  function button(label, svg, parent, action) {
    var b = element("button", "width:22px;height:22px;margin:0 3px;padding:0;display:flex;justify-content:center;" +
      "align-items:center;border:none;border-radius:4px;cursor:pointer;", parent);
    b.type = "button";
    b.title = label;
    b.innerHTML = svg;
    b.addEventListener("mouseover", function () {
      if (!b.getAttribute("data-selected")) b.style.backgroundColor = BUTTON_HOVER;
    });
    b.addEventListener("mouseout", function () {
      if (!b.getAttribute("data-selected")) b.style.backgroundColor = "transparent";
    });
    b.addEventListener("click", action);
    setButton(b, false);
    return b;
  }

  function selectTool(plot, name) {
    var zoom = plot.zoom;
    zoom.tool = zoom.tool === name ? null : name;
    zoom.buttons.forEach(function (entry) { setButton(entry.button, entry.name === zoom.tool); });
    plot.svg.style.cursor = zoom.tool === "pan" ? "move" : zoom.tool ? "crosshair" : "";
  }

  function toolbar(plot) {
    var holder = element("div", "display:flex;justify-content:center;align-items:center;margin:0 0 5px 0;line-height:normal;");
    holder.className = "ggrust-toolbar";
    var bar = element("div", "position:relative;display:inline-flex;justify-content:center;align-items:center;" +
      "box-sizing:border-box;padding:2px 5px;background-color:" + BAR_BACKGROUND +
      ";border:1px solid rgb(200, 200, 200);border-radius:8px;", holder);
    plot.zoom.buttons = TOOLS.map(function (tool) {
      return { name: tool.name, button: button(tool.label, tool.icon, bar, function () { selectTool(plot, tool.name); }) };
    });
    button("Reset", RESET_ICON, bar, function () {
      if (plot.zoom.tool) selectTool(plot, plot.zoom.tool);
      resetViews(plot);
    });
    plot.root.insertBefore(holder, plot.svg);
  }

  // Set up zoom/pan when the engine wrote a zoom payload (`ggtb()`).
  function attachZoom(plot) {
    var data = plot.root.querySelector("script.ggrust-zoom");
    if (!data || !plot.svg) return;
    var payload;
    try {
      payload = JSON.parse(data.textContent);
    } catch (error) {
      return;
    }
    var tileElements = plot.svg.querySelectorAll("g.tile");
    var next = 0;
    plot.zoom = {
      payload: payload,
      tool: null,
      drag: null,
      plots: payload.plots.map(function (zoomPlot) {
        return {
          shared: zoomPlot.shared,
          tiles: zoomPlot.tiles.map(function (spec) {
            var tile = { spec: spec, element: tileElements[next++] || null };
            tile.views = { h: spec.horizontal.domain.slice(), v: spec.vertical.domain.slice() };
            return tile;
          }).filter(function (tile) { return tile.element !== null; }),
        };
      }),
    };
    if (!plot.width) {
      var viewBox = plot.svg.viewBox && plot.svg.viewBox.baseVal;
      plot.width = viewBox && viewBox.width ? viewBox.width : plot.svg.width.baseVal.value;
      plot.height = viewBox && viewBox.height ? viewBox.height : plot.svg.height.baseVal.value;
    }
    // Strokes keep their width when a panel's marks are scaled.
    var style = document.createElementNS(SVG_NS, "style");
    style.textContent = "#" + plot.id + " .panel * { vector-effect: non-scaling-stroke; }";
    plot.svg.insertBefore(style, plot.svg.firstChild);
    plot.zoom.decorations = document.createElementNS(SVG_NS, "g");
    plot.zoom.decorations.setAttribute("class", "ggrust-selection");
    plot.svg.appendChild(plot.zoom.decorations);
    toolbar(plot);
    if (plot.overlay) {
      plot.overlay.style.top = plot.svg.offsetTop + "px";
    }
    plot.svg.addEventListener("mousedown", function (event) { onPress(plot, event); });
    plot.svg.addEventListener("wheel", function (event) { onWheel(plot, event); }, { passive: false });
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
      if (plot.dragging) {
        return;
      }
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
    attachZoom(plot);
    return plot;
  }

  // `breaks` is the zoom's break generator, exposed for tests.
  window.ggrust = { version: version, mount: mount, lookup: lookup, breaks: flexBreaks };
})
