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
// centerpoint zoom, reset) and zooms panels: see "Zoom and pan" below. With
// `ggtb(layers=...)` the toolbar also hides and shows layers: see "Layer toggles".
// Dense point batches (more than 5k markers) arrive as base64 typed arrays
// and are stamped on a canvas inside their SVG placeholder: see "Canvas points".
// Clicking a discrete legend key hides or shows its marks and their tooltips:
// see "Interactive legend". Animations (gganimate transitions) get a player
// instead of all that: see "Animation player".
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
    var list = layer.cells.get(key) || [];
    var keyed = layer.canvasKeys ? layer.canvasKeys.hiddenKeys : layer.hiddenKeys;
    return keyed ? list.filter(function (i) { return !keyHidden(layer, i); }) : list;
  }

  // Whether target `i` of `layer` belongs to a legend key switched off. Targets
  // that are the markers of a keyed canvas batch take its key per marker.
  function keyHidden(layer, i) {
    var canvas = layer.canvasKeys;
    if (canvas) return !!canvas.hiddenKeys && canvas.hiddenKeys[canvas.key[i]];
    return !!layer.hiddenKeys && layer.hiddenKeys[layer.keys.rows[layer.targets.rows[i]]];
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
    if (!layer.cells) index(layer); // moved targets are indexed on first use
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
        if (keyHidden(layer, vertices[0])) return; // a line is one group, so one key
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
      if (layer.hidden || !inPanel(layer, x, y)) {
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

  // lets-plot side tooltips (boxplot statistics): a dark box per statistic,
  // its pointer from the box edge to the cursor at the statistic's position;
  // stacked without overlapping, left of (or above, flipped) the cursor.
  var SIDE_FILL = "#474747";
  var SIDE_POINTER = 12; // px from the box edge to the pointer's tip
  function showSide(layer, lead, scale, width, height, overlay) {
    var items = [];
    (layer.side_tooltips || []).forEach(function (line) {
      var value = text(line.text, lead.row);
      var at = line.at[lead.row];
      if (value === null || at === null || at === undefined) {
        return;
      }
      var node = element(
        "div",
        "position:absolute;left:0;top:0;box-sizing:border-box;background:" + SIDE_FILL +
          ";color:#fff;border:1px solid #fff;border-radius:4px;padding:6px 6px;font:" + FONT +
          ";line-height:13px;white-space:nowrap;pointer-events:none;",
        overlay
      );
      node.textContent = value;
      items.push({ node: node, at: at * scale, w: node.offsetWidth, h: node.offsetHeight, horizontal: line.horizontal });
    });
    if (!items.length) {
      return;
    }
    var horizontal = items[0].horizontal;
    var cursor = horizontal ? lead.y * scale : lead.x * scale;
    items.sort(function (a, b) {
      return a.at - b.at;
    });
    // Along the value axis: centred on the statistic, pushed apart, kept inside.
    var size = function (item) { return horizontal ? item.w : item.h; };
    var limit = horizontal ? width : height;
    var next = 0;
    items.forEach(function (item) {
      item.start = Math.max(item.at - size(item) / 2, next);
      next = item.start + size(item) + GAP;
    });
    var overflow = next - GAP - limit;
    if (overflow > 0) {
      items.forEach(function (item) {
        item.start = Math.max(0, item.start - overflow);
      });
    }
    var across = Math.max.apply(null, items.map(function (item) { return horizontal ? item.h : item.w; }));
    var before = cursor - SIDE_POINTER - across >= 0;
    var svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("width", width);
    svg.setAttribute("height", height);
    svg.style.cssText = "position:absolute;left:0;top:0;overflow:visible;pointer-events:none;";
    overlay.insertBefore(svg, overlay.firstChild);
    items.forEach(function (item) {
      var w = item.w, h = item.h, x, y, points;
      if (!horizontal) {
        x = before ? cursor - SIDE_POINTER - w : cursor + SIDE_POINTER;
        y = item.start;
        var edge = before ? x + w - 1 : x + 1;
        points = [[edge, y + h * 0.3], [cursor, item.at], [edge, y + h * 0.7]];
      } else {
        y = before ? cursor - SIDE_POINTER - h : cursor + SIDE_POINTER;
        x = item.start;
        var edgeY = before ? y + h - 1 : y + 1;
        points = [[x + w * 0.3, edgeY], [item.at, cursor], [x + w * 0.7, edgeY]];
      }
      item.node.style.transform = "translate(" + x + "px," + y + "px)";
      var pointer = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
      pointer.setAttribute("points", points.map(function (p) { return p.join(","); }).join(" "));
      pointer.setAttribute("fill", SIDE_FILL);
      pointer.setAttribute("stroke", "#fff");
      svg.appendChild(pointer);
    });
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
    showSide(layer, lead, scale, width, height, overlay);
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

  // ---- Canvas points (crates/ggrust-core/src/render/canvas.rs) ----

  // A dense batch is an empty `<foreignObject class="ggrust-canvas">` over the
  // whole scene, at the batch's place in the SVG (z-order and clip paths hold).
  // Its payload: `xy` Float32 centres, `styles` (size, paints and marker parts
  // around 0,0) and `style` per-marker indices. Each style is rendered once to
  // a small sprite per quarter-pixel offset (stamped at whole pixels, so
  // markers stay as crisp as the SVG's), then stamped per marker in data order,
  // so translucent markers stack as in the SVG. Past MAX_SPRITES styles markers
  // are traced one by one instead.

  var XHTML_NS = "http://www.w3.org/1999/xhtml";
  var MAX_SPRITES = 4096;
  var SUBPIXEL = 4; // sprite offsets per device pixel and axis

  function decodeBase64(text) {
    var binary = window.atob(text);
    var bytes = new Uint8Array(binary.length);
    for (var i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes.buffer;
  }

  function tracePart(context, part, k) {
    context.beginPath();
    if (part[0] === "c") {
      context.arc(part[1] * k, part[2] * k, part[3] * k, 0, 2 * Math.PI);
    } else if (part[0] === "r") {
      context.rect(part[1] * k, part[2] * k, part[3] * k, part[4] * k);
    } else if (part[0] === "p") {
      context.moveTo(part[1] * k, part[2] * k);
      for (var i = 3; i + 1 < part.length; i += 2) context.lineTo(part[i] * k, part[i + 1] * k);
      context.closePath();
    } else if (part[0] === "l") {
      context.moveTo(part[1] * k, part[2] * k);
      context.lineTo(part[3] * k, part[4] * k);
    }
  }

  // Paint a marker centred at the context's origin: geometry grows with `k`,
  // the outline keeps its width (as the SVG's non-scaling strokes).
  function paintMarker(context, style, k) {
    style.parts.forEach(function (part) {
      tracePart(context, part, k);
      if (style.fill && part[0] !== "l") {
        context.fillStyle = style.fill;
        context.fill();
      }
      if (style.stroke) {
        context.strokeStyle = style.stroke;
        context.lineWidth = style.width;
        context.stroke();
      }
    });
  }

  // Half the side of the sprite of `style` in device pixels.
  function spriteHalf(style, k, ratio) {
    var reach = 0;
    style.parts.forEach(function (part) {
      var extent = part[0] === "c" ? [Math.abs(part[1]) + part[3], Math.abs(part[2]) + part[3]]
        : part[0] === "r" ? [Math.abs(part[1]), Math.abs(part[2]), Math.abs(part[1] + part[3]), Math.abs(part[2] + part[4])]
        : part.slice(1).map(Math.abs);
      extent.forEach(function (value) { reach = Math.max(reach, value); });
    });
    // A miter reaches a full stroke width past a 60 degree corner (triangles).
    return Math.ceil((reach * k + style.width + 1) * ratio);
  }

  // The sprites of one marker scale `k` share atlas pages (a canvas each is
  // costly to create, and a zoom with `size_zoomin` changes `k` every step).
  // Pages are at most ATLAS_PAGE device pixels a side, sized to what the
  // layer's styles can need; a larger sprite gets a page of its own.
  var ATLAS_PAGE = 1024;

  function newAtlas(layer, k) {
    var area = 0;
    layer.styles.forEach(function (style) {
      var side = 2 * spriteHalf(style, k, layer.ratio);
      area += SUBPIXEL * SUBPIXEL * side * side;
    });
    return { k: k, side: Math.min(ATLAS_PAGE, Math.ceil(Math.sqrt(area * 1.25))), page: null, x: 0, y: 0, row: 0 };
  }

  function newPage(width, height) {
    var canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    return { canvas: canvas, context: canvas.getContext("2d") };
  }

  // `style` drawn with its centre at (half + dx, half + dy) device pixels of
  // its cell: {canvas, x, y (the cell's corner), half}.
  function sprite(atlas, style, k, ratio, dx, dy) {
    var half = spriteHalf(style, k, ratio), size = 2 * half;
    var page, x = 0, y = 0;
    if (size > atlas.side) {
      page = newPage(size, size);
    } else {
      if (atlas.page && atlas.x + size > atlas.side) {
        atlas.x = 0;
        atlas.y += atlas.row;
        atlas.row = 0;
      }
      if (!atlas.page || atlas.y + size > atlas.side) {
        atlas.page = newPage(atlas.side, atlas.side);
        atlas.x = atlas.y = atlas.row = 0;
      }
      page = atlas.page;
      x = atlas.x;
      y = atlas.y;
      atlas.x += size;
      atlas.row = Math.max(atlas.row, size);
    }
    page.context.setTransform(ratio, 0, 0, ratio, x + half + dx, y + half + dy);
    paintMarker(page.context, style, k);
    return { canvas: page.canvas, x: x, y: y, half: half };
  }

  function attachCanvas(plot) {
    var data = plot.root.querySelector('script[type="application/vnd.ggrust-canvas+json"]');
    if (!data || !plot.svg) return;
    var payload;
    try {
      payload = JSON.parse(data.textContent);
    } catch (error) {
      return;
    }
    var holders = plot.svg.querySelectorAll("foreignObject.ggrust-canvas");
    plot.canvases = [];
    plot.canvasXy = {};
    plot.canvasLayers = {};
    for (var i = 0; i < holders.length; i++) {
      var number = +holders[i].getAttribute("data-batch");
      var batch = payload[number];
      if (!batch) continue;
      var width = +holders[i].getAttribute("width"), height = +holders[i].getAttribute("height");
      var ratio = window.devicePixelRatio || 1;
      var canvas = document.createElementNS(XHTML_NS, "canvas");
      canvas.width = Math.ceil(width * ratio);
      canvas.height = Math.ceil(height * ratio);
      canvas.setAttribute("style", "display:block;pointer-events:none;width:" + width + "px;height:" + height + "px");
      holders[i].appendChild(canvas);
      var layer = {
        holder: holders[i],
        canvas: canvas,
        ratio: ratio,
        count: batch.count,
        xy: new Float32Array(decodeBase64(batch.xy)),
        styles: batch.styles,
        style: batch.style ? new (batch.style_type === "u32" ? Uint32Array : Uint16Array)(decodeBase64(batch.style)) : null,
        // Interactive legend: per marker, its index into `keys` (key id lists).
        key: batch.key ? new Uint8Array(decodeBase64(batch.key)) : null,
        keys: batch.keys || null,
        hiddenKeys: null,
        sprites: {},
      };
      holders[i].ggrustCanvas = layer;
      plot.canvases.push(layer);
      plot.canvasXy[number] = layer.xy;
      plot.canvasLayers[number] = layer;
      drawCanvas(layer, { scale: 1, offset: 0 }, { scale: 1, offset: 0 }, 1);
    }
  }

  // Draw the markers of `layer` through the per-axis maps `h`, `v` (unzoomed
  // pixel -> pixel), with marker geometry scaled by `k`.
  function drawCanvas(layer, h, v, k) {
    layer.view = [h, v, k];
    layer.drawnHiddenKeys = layer.hiddenKeys;
    var context = layer.canvas.getContext("2d");
    var ratio = layer.ratio, xy = layer.xy, style = layer.style;
    var markerKey = layer.key, hiddenKeys = layer.hiddenKeys;
    var width = layer.canvas.width, height = layer.canvas.height;
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, width, height);
    if (layer.hidden) return;
    var direct = layer.styles.length > MAX_SPRITES;
    if (!direct && layer.sprites.k !== k) layer.sprites = { k: k, atlas: newAtlas(layer, k) };
    var hs = h.scale * ratio, ho = h.offset * ratio, vs = v.scale * ratio, vo = v.offset * ratio;
    // Sprites are all painted before the first stamp: painting an atlas page
    // between stamps makes the browser sync it with the canvas every time.
    if (!layer.stamps || layer.stamps.length < 3 * layer.count) layer.stamps = new Int32Array(3 * layer.count);
    var stamps = layer.stamps, visible = 0;
    for (var i = 0; i < layer.count; i++) {
      if (hiddenKeys && hiddenKeys[markerKey[i]]) continue;
      var x = hs * xy[2 * i] + ho, y = vs * xy[2 * i + 1] + vo;
      var s = style ? style[i] : 0;
      // Off-canvas markers are skipped first (zoomed pixels can pass 2^31).
      var reach = (layer.styles[s].size * k + layer.styles[s].width + 2) * ratio;
      if (!(x > -reach && y > -reach && x < width + reach && y < height + reach)) continue;
      if (direct) {
        context.setTransform(ratio, 0, 0, ratio, x, y);
        paintMarker(context, layer.styles[s], k);
        continue;
      }
      // The nearest quarter pixel: whole pixel + sprite offset.
      x = Math.round(x * SUBPIXEL);
      y = Math.round(y * SUBPIXEL);
      var fx = x & (SUBPIXEL - 1), fy = y & (SUBPIXEL - 1);
      var key = (s * SUBPIXEL + fx) * SUBPIXEL + fy;
      if (!layer.sprites[key]) {
        layer.sprites[key] = sprite(layer.sprites.atlas, layer.styles[s], k, ratio, fx / SUBPIXEL, fy / SUBPIXEL);
      }
      stamps[3 * visible] = (x - fx) / SUBPIXEL;
      stamps[3 * visible + 1] = (y - fy) / SUBPIXEL;
      stamps[3 * visible + 2] = key;
      visible++;
    }
    for (var j = 0; j < visible; j++) {
      var stamp = layer.sprites[stamps[3 * j + 2]];
      var side = 2 * stamp.half;
      context.drawImage(stamp.canvas, stamp.x, stamp.y, side, side,
        stamps[3 * j] - stamp.half, stamps[3 * j + 1] - stamp.half, side, side);
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
  // Toolbar style ("soft frost ink", agreed with the user; lets-plot's
  // DefaultToolbarJs uses a grey bordered bar and a blue selected button).
  var BAR_BACKGROUND = "rgba(255, 255, 255, 0.85)";
  var BAR_BORDER = "0.5px solid rgba(0, 0, 0, 0.12)";
  var BAR_SHADOW = "0 1px 2px rgba(0, 0, 0, 0.06)";
  var BUTTON_STROKE = "rgb(95, 94, 90)";
  var BUTTON_HOVER = "rgba(0, 0, 0, 0.05)";
  var BUTTON_SELECTED = "rgb(44, 44, 42)";
  var MIN_SELECTION = 15; // px (DrawRectFeedback.MIN_SIZE)
  var WHEEL_STEP = 0.08; // WheelZoomFeedback: one wheel notch
  var MIN_EXPONENT = -7, MAX_EXPONENT = 6; // number format: decimal notation range
  var MAX_DECIMAL_PLACES = 12;
  var MIN_POSITIVE = 4.9e-324 * 1e5, MAX_VALUE = Number.MAX_VALUE / 1e5;
  var LOG_MAX_VALUE = Number.MAX_VALUE; // logs accept every finite value, as ggplot2

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
    return [Math.log(MIN_POSITIVE) / Math.log(base), Math.log(LOG_MAX_VALUE) / Math.log(base)];
  }

  // Whether a transformed position is one the transform produces (`is_in_range`).
  function inRange(transform, position) {
    if (!isFinite(position)) return false;
    if (transform === "sqrt") return position >= 0;
    if (transform === "exp" || transform === "reciprocal") return position > 0;
    var box = boxCox(transform);
    if (box && Math.abs(box[0]) >= BOX_COX_LOG_POWER) return position * box[0] + 1 >= 0;
    return true;
  }

  function inDomain(transform, value) {
    if (!isFinite(value)) return false;
    if (transform === "sqrt") return value >= 0;
    if (transform === "log10" || transform === "log2" || transform === "log") return value >= MIN_POSITIVE && value <= LOG_MAX_VALUE;
    if (transform === "log1p") return value > -1;
    if (transform === "exp") return Math.exp(value) <= MAX_VALUE;
    if (transform === "logit" || transform === "probit") return value > 0 && value < 1;
    if (transform === "reciprocal") return value > 0;
    var box = boxCox(transform);
    if (box) {
      var shifted = value + box[1];
      return shifted > 0 || (shifted === 0 && box[0] >= BOX_COX_LOG_POWER);
    }
    return true;
  }

  var BOX_COX_LOG_POWER = 1e-7;

  // [power, offset] of a "boxcox(p,offset)" transform name, otherwise null.
  function boxCox(transform) {
    var match = /^boxcox\((.*),(.*)\)$/.exec(transform);
    return match ? [parseFloat(match[1]), parseFloat(match[2])] : null;
  }

  // Standard normal CDF: Taylor series near 0, continued fraction in the tails.
  function normalLower(z) {
    if (z < 0) return 1 - normalLower(-z);
    var density = Math.exp(-z * z / 2) / Math.sqrt(2 * Math.PI);
    if (z < 3) {
      var term = z, sum = z;
      for (var i = 3; i < 200 && Math.abs(term) > 1e-17; i += 2) {
        term *= z * z / i;
        sum += term;
      }
      return 0.5 + density * sum;
    }
    var fraction = 0;
    for (var k = 200; k >= 1; k--) fraction = k / (z + fraction);
    return 1 - density / (z + fraction);
  }

  // Normal quantile by bisection: only a few breaks per zoom step need it.
  function normalQuantile(p) {
    var low = -40, high = 40;
    for (var i = 0; i < 100; i++) {
      var middle = (low + high) / 2;
      if (normalLower(middle) < p) low = middle;
      else high = middle;
    }
    return (low + high) / 2;
  }

  function applyTransform(transform, value) {
    if (!inDomain(transform, value)) return null;
    switch (transform) {
      case "reverse": return -value;
      case "sqrt": return Math.sqrt(value);
      case "log10":
      case "log2":
      case "log": {
        var base = transform === "log10" ? 10 : transform === "log2" ? 2 : Math.E;
        var limits = logLimits(base);
        var log = Math.log(value) / Math.log(base);
        return isNaN(log) ? log : clamp(log, limits[0], limits[1]);
      }
      case "log1p": return Math.log1p(value);
      case "exp": return Math.exp(value);
      case "logit": return Math.log(value / (1 - value));
      case "probit": return normalQuantile(value);
      case "reciprocal": return 1 / value;
      case "pseudo_log": return Math.asinh(value / 2);
      case "symlog":
        return Math.abs(value) < 1 ? value : Math.sign(value) * (1 + Math.log(Math.abs(value)) / Math.log(10));
      default: {
        var box = boxCox(transform);
        if (!box) return value;
        if (Math.abs(box[0]) < BOX_COX_LOG_POWER) return Math.log(value + box[1]);
        return (Math.pow(value + box[1], box[0]) - 1) / box[0];
      }
    }
  }

  function inverseTransform(transform, value) {
    switch (transform) {
      case "reverse": return -value;
      case "sqrt": return value * value;
      case "log10":
      case "log2":
      case "log": {
        var base = transform === "log10" ? 10 : transform === "log2" ? 2 : Math.E;
        var limits = logLimits(base);
        return Math.min(Math.pow(base, clamp(value, limits[0], limits[1])), LOG_MAX_VALUE);
      }
      case "log1p": return Math.expm1(value);
      case "exp": return Math.log(Math.max(value, MIN_POSITIVE));
      case "logit": return 1 / (1 + Math.exp(-value));
      case "probit": return normalLower(value);
      case "reciprocal": return 1 / value;
      case "pseudo_log": return 2 * Math.sinh(value);
      case "symlog":
        return Math.abs(value) < 1 ? value : Math.sign(value) * Math.pow(10, Math.abs(value) - 1);
      default: {
        var box = boxCox(transform);
        if (!box) return value;
        if (Math.abs(box[0]) < BOX_COX_LOG_POWER) return Math.exp(value) - box[1];
        return Math.pow(value * box[0] + 1, 1 / box[0]) - box[1];
      }
    }
  }

  // -- Labels (crates/ggrust-core/src/scale/format.rs) --

  // The theme's `exponent_format` ({style: "e"|"pow"|"pow_full", min, max}, null
  // bounds keep the defaults), as `ExponentFormat` in Rust.
  function exponentBounds(exponent) {
    var min = exponent && exponent.min !== null && exponent.min !== undefined ? exponent.min : MIN_EXPONENT;
    var max = exponent && exponent.max !== null && exponent.max !== undefined ? exponent.max : MAX_EXPONENT;
    return [min, max];
  }

  function precisionOf(reference, step, exponent) {
    var bounds = exponentBounds(exponent);
    if (reference === 0) reference = 2.2250738585072014e-308 * 10;
    if (step === 0) step = reference / 10;
    var domainPower = Math.log10(reference), stepPower = Math.log10(step);
    var scientific = (domainPower < 0 && stepPower <= bounds[0]) ||
      (domainPower >= bounds[1] && stepPower > 2);
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
  function formatGeneral(value, precision, exponentFormat) {
    if (!isFinite(value)) {
      return { text: value !== value ? "NaN" : value > 0 ? "inf" : "-inf", superscript: null };
    }
    var rounded = roundedDigits(value, Math.max(precision - 1, 0));
    var digits = rounded.digits, exponent = rounded.exponent;
    if (/^0*$/.test(digits)) {
      return { text: "0", superscript: null };
    }
    var sign = value < 0 ? "-" : "";
    var bounds = exponentBounds(exponentFormat);
    if (exponent > bounds[0] && exponent < bounds[1]) {
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
    var style = exponentFormat ? exponentFormat.style : "pow";
    if (style === "e") {
      return { text: sign + mantissa + "e" + (exponent >= 0 ? "+" : "") + exponent, superscript: null };
    }
    return {
      text: mantissa === "1" && style === "pow" ? sign + "10" : sign + mantissa + "·10",
      superscript: String(exponent).replace("-", "−"),
    };
  }

  function formatBreaks(values, exponent) {
    var reference, step;
    if (values.length === 0) {
      reference = 0; step = 0.5;
    } else if (values.length === 1) {
      reference = Math.abs(values[0]); step = reference / 10;
    } else {
      reference = Math.max(Math.abs(values[0]), Math.abs(values[values.length - 1]));
      step = Math.abs(values[1] - values[0]);
    }
    var precision = precisionOf(reference, step, exponent);
    return values.map(function (value) { return formatGeneral(value, precision, exponent); });
  }

  function formatIndividually(values, exponent) {
    return values.map(function (value, index) {
      var step = values.length === 1 ? value / 10 : index === 0 ? value - values[1] : value - values[index - 1];
      return formatGeneral(value, precisionOf(Math.abs(value), Math.abs(step), exponent), exponent);
    });
  }

  // Generated breaks of an axis over [low, high] (transformed units), as
  // `ContinuousPositionScale::generated_breaks`: {positions, labels}.
  function generatedBreaks(transform, low, high, count, exponent) {
    var a = inverseTransform(transform, low), b = inverseTransform(transform, high);
    var dataLow = Math.min(a, b), dataHigh = Math.max(a, b);
    var values;
    if (transform === "identity" || transform === "reverse") {
      values = linearBreaks(dataLow, dataHigh, count);
      return {
        positions: values.map(function (v) { return inverseTransform(transform, v); }),
        labels: formatBreaks(values, exponent),
      };
    }
    // Straight from the domain, as the engine: a padded end can lie outside the
    // transform's range (sqrt below 0), where a round trip would fold it back.
    var lowT = Math.min(low, high), highT = Math.max(low, high);
    if (transform === "log10" || transform === "symlog") {
      var powers = Math.round(Math.floor(highT) - Math.ceil(lowT)) + 1;
      if (powers >= 3 && powers <= count) {
        count = powers;
      }
    }
    values = linearBreaks(lowT, highT, count)
      .filter(function (p) { return inRange(transform, p); })
      .map(function (p) { return inverseTransform(transform, p); })
      .filter(function (v) { return inDomain(transform, v); });
    return {
      positions: values.map(function (v) {
        var p = applyTransform(transform, v);
        return p === null ? NaN : p;
      }),
      labels: formatIndividually(values, exponent),
    };
  }

  // lets-plot `getFlexBreaks`: at most `count` breaks.
  function flexBreaks(transform, low, high, count, exponent) {
    var breaks = generatedBreaks(transform, low, high, count, exponent);
    if (count === 1 && breaks.positions.length) {
      return { positions: breaks.positions.slice(0, 1), labels: breaks.labels.slice(0, 1) };
    }
    var target = count;
    while (breaks.positions.length > count) {
      var delta = Math.max(Math.floor((breaks.positions.length - count) / 2), 1);
      if (target <= delta + 1) {
        return generatedBreaks(transform, low, high, 1, exponent);
      }
      target -= delta;
      breaks = generatedBreaks(transform, low, high, target, exponent);
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
    var breaks = flexBreaks(axis.transform, low, high, Math.max(axis.count, 1), axis.exponent);
    breaks.positions.forEach(function (position, index) {
      var label = breaks.labels[index];
      var template = label.superscript === null ? axis.label : axis.label_superscript;
      // Function replacements: a `$` in a label is literal.
      var svg = template && template
        // Global: an axis drawn on two sides has a label per side in its template.
        .replace(new RegExp(">@<", "g"), function () { return ">" + escapeXml(label.text) + "<"; })
        .replace(new RegExp(">\\^<", "g"), function () { return ">" + escapeXml(label.superscript || "") + "<"; });
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
    // Minor grid lines and minor ticks (template `which`) between the breaks.
    var minor = function (axis, view, breaks, horizontal, which, out) {
      if (!axis[which] || breaks.length < 2) return;
      var step = breaks[1].position - breaks[0].position;
      var start = breaks[0].position - step / 2;
      var from = horizontal ? panel[0] : panel[1], to = from + (horizontal ? panel[2] : panel[3]);
      for (var i = 0; i <= breaks.length; i++) {
        var pixel = pixelOf(axis, view, start + i * step);
        if (pixel >= from && pixel <= to) out.push(translated(axis[which], horizontal, pixel));
      }
    };
    var grid = [];
    minor(spec.horizontal, views.h, h, true, "minor", grid);
    minor(spec.vertical, views.v, v, false, "minor", grid);
    [[spec.horizontal, h, true], [spec.vertical, v, false]].forEach(function (entry) {
      if (entry[0].major) {
        entry[1].forEach(function (b) { grid.push(translated(entry[0].major, entry[2], b.pixel)); });
      }
    });
    var axisNodes = function (axis, view, breaks, horizontal) {
      var out = axis.line ? [axis.line] : [];
      breaks.forEach(function (b) {
        if (axis.tick) out.push(translated(axis.tick, horizontal, b.pixel));
        if (b.label) out.push(translated(b.label, horizontal, b.shift));
      });
      minor(axis, view, breaks, horizontal, "minor_tick", out);
      return out.join("");
    };
    // Secondary axes (sec_axis): fixed breaks moving with the primary view.
    var secondary = function (index, view, horizontal) {
      var axis = spec.secondary && spec.secondary[index];
      return axis ? axisNodes(axis, view, visibleBreaks(axis, view), horizontal) : "";
    };
    return {
      grid: grid.join(""),
      x: axisNodes(spec.horizontal, views.h, h, true) + secondary(0, views.h, true),
      y: axisNodes(spec.vertical, views.v, v, false) + secondary(1, views.v, false)
    };
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
      if (child.getAttribute("class") === "layer" || child.getAttribute("class") === "keyed") {
        collectMarks(child, marks); // one layer's (or legend key's) marks (see "Layer toggles")
      } else if (child.getAttribute("class") === "ggrust-canvas") {
        if (child.ggrustCanvas) marks.push({ kind: "canvas", layer: child.ggrustCanvas });
      } else if (child.getAttribute("class") === "ggrust-points") {
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
      } else if (child.tagName.toLowerCase() === "g" && child.querySelector(".ggrust-points,.ggrust-canvas,[data-anchor]")) {
        collectMarks(child, marks);
      } else {
        // A shape may carry its own transform (a mirrored image): zoom composes with it.
        marks.push({ kind: "shape", element: child, transform: child.getAttribute("transform") });
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
      tile.trees = treeGroups(element);
      tile.nested = nestedGroups(element);
    }
    var reset = isUnzoomed(tile);
    var h = affine(tile.spec.horizontal, tile.views.h), v = affine(tile.spec.vertical, tile.views.v);
    var k = reset ? 1 : sizeFactor(plot.zoom, tile);
    var matrix = "matrix(" + h.scale + " 0 0 " + v.scale + " " + h.offset + " " + v.offset + ")";
    tile.marks.forEach(function (mark) {
      if (mark.kind === "canvas") {
        drawCanvas(mark.layer, h, v, k);
        return;
      }
      var e = mark.element;
      if (mark.kind === "shape") {
        var own = mark.transform;
        if (reset) {
          if (own) e.setAttribute("transform", own);
          else e.removeAttribute("transform");
        } else {
          e.setAttribute("transform", own ? matrix + " " + own : matrix);
        }
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
    moveTrees(tile.trees, h, v, reset);
    moveNested(tile.nested, h, v, reset, tile.spec.panel);
    var content = reset ? tile.saved : axisContent(tile);
    Object.keys(tile.groups).forEach(function (key) {
      if (tile.groups[key]) tile.groups[key].innerHTML = content[key] || "";
    });
    moveTargets(plot, tile, h, v, k, reset);
  }

  // Axis dendrograms ("dendrogram-x" / "dendrogram-y" beside the axes) and the
  // layers of side panels ("side-x" / "side-y", bands sharing the horizontal or
  // vertical axis): their marks go into an inner group, moved along that axis
  // only (leaves and tracks are linear in it, the other axis stays); the outer
  // group keeps the engine's clip.
  function treeGroups(element) {
    var trees = [];
    [["dendrogram-x", true], ["dendrogram-y", false], ["side-x", true], ["side-y", false]].forEach(function (entry) {
      for (var child = element.firstElementChild; child; child = child.nextElementSibling) {
        if (child.getAttribute("class") !== entry[0]) continue;
        var inner = document.createElementNS(SVG_NS, "g");
        while (child.firstChild) inner.appendChild(child.firstChild);
        child.appendChild(inner);
        trees.push({ inner: inner, horizontal: entry[1] });
      }
    });
    return trees;
  }

  function moveTrees(trees, h, v, reset) {
    (trees || []).forEach(function (tree) {
      if (reset) {
        tree.inner.removeAttribute("transform");
      } else if (tree.horizontal) {
        tree.inner.setAttribute("transform", "matrix(" + h.scale + " 0 0 1 " + h.offset + " 0)");
      } else {
        tree.inner.setAttribute("transform", "matrix(1 0 0 " + v.scale + " 0 " + v.offset + ")");
      }
    });
  }

  // Nested axis guides ("nested-x" / "nested-y", guide_axis_nested): brackets
  // (inside a clipped group) scale along their axis, names move with the centre
  // of their group and hide once it leaves the panel.
  function nestedGroups(element) {
    var groups = [];
    for (var child = element.firstElementChild; child; child = child.nextElementSibling) {
      var name = child.getAttribute("class");
      if (name !== "nested-x" && name !== "nested-y") continue;
      var group = { horizontal: name === "nested-x", shapes: [], labels: [] };
      for (var part = child.firstElementChild; part; part = part.nextElementSibling) {
        if (part.hasAttribute("data-anchor")) group.labels.push(anchored(part, "anchor"));
        else if (part.firstElementChild) group.shapes.push(part.firstElementChild);
      }
      groups.push(group);
    }
    return groups;
  }

  function moveNested(groups, h, v, reset, panel) {
    (groups || []).forEach(function (group) {
      var axis = group.horizontal ? h : v;
      group.shapes.forEach(function (shape) {
        if (reset) shape.removeAttribute("transform");
        else if (group.horizontal) shape.setAttribute("transform", "matrix(" + h.scale + " 0 0 1 " + h.offset + " 0)");
        else shape.setAttribute("transform", "matrix(1 0 0 " + v.scale + " 0 " + v.offset + ")");
      });
      var low = group.horizontal ? panel[0] : panel[1];
      var high = low + (group.horizontal ? panel[2] : panel[3]);
      group.labels.forEach(function (label) {
        if (reset) {
          label.element.removeAttribute("transform");
          label.element.removeAttribute("display");
          return;
        }
        var at = group.horizontal ? label.x : label.y;
        var moved = axis.scale * at + axis.offset;
        label.element.setAttribute("transform", group.horizontal ? "translate(" + (moved - at) + " 0)" : "translate(0 " + (moved - at) + ")");
        if (moved < low - 0.5 || moved > high + 0.5) label.element.setAttribute("display", "none");
        else label.element.removeAttribute("display");
      });
    });
  }

  function samePanel(a, b) {
    for (var i = 0; i < 4; i++) {
      if (Math.abs(a[i] - b[i]) > 0.01) return false;
    }
    return true;
  }

  // Tooltip targets of the layers in `tile`'s panel, through the same mapping.
  function moveTargets(plot, tile, h, v, k, reset) {
    var p = tile.spec.panel;
    (plot.layers || []).forEach(function (layer) {
      // A side panel's layer moves along the axis it shares with the panel only.
      var q = layer.panel, along = null;
      if (layer.side === "x" && Math.abs(q[0] - p[0]) < 0.01 && Math.abs(q[2] - p[2]) < 0.01) along = "x";
      if (layer.side === "y" && Math.abs(q[1] - p[1]) < 0.01 && Math.abs(q[3] - p[3]) < 0.01) along = "y";
      if (!along && !samePanel(q, p)) return;
      if (layer.side && !along) return;
      if (!layer.original) layer.original = layer.targets;
      var o = layer.original;
      // Indexed again on the next lookup, not on every zoom step.
      layer.cells = null;
      if (reset) {
        layer.targets = o;
        return;
      }
      var hs = along === "y" ? 1 : h.scale, ho = along === "y" ? 0 : h.offset;
      var vs = along === "x" ? 1 : v.scale, vo = along === "x" ? 0 : v.offset;
      var mx = function (value) { return hs * value + ho; };
      var my = function (value) { return vs * value + vo; };
      var t = {};
      Object.keys(o).forEach(function (key) { t[key] = o[key]; });
      if (o.kind === "polygons") {
        t.rings = o.rings.map(function (ring) {
          return ring.map(function (value, i) { return i % 2 ? my(value) : mx(value); });
        });
      } else {
        t.x = o.x.map(mx);
        t.y = o.y.map(my);
        if (o.kind === "points" && !along) t.radius = o.radius.map(function (r) { return r * k; });
        if (o.kind === "rects") {
          t.width = o.width.map(function (w) { return w * hs; });
          t.height = o.height.map(function (value) { return value * vs; });
        }
      }
      layer.targets = t;
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
  // A fixed aspect panel (`coord_fixed`) has no bands, and its box widens to the
  // panel's shape so both axes zoom equally (deviations.md).
  function selection(drag, to, centre) {
    var r = drag.bounds, from = drag.from;
    var limit = function (p) {
      return [clamp(p[0], r[0], r[0] + r[2]), clamp(p[1], r[1], r[1] + r[3])];
    };
    var dx = to[0] - from[0], dy = to[1] - from[1];
    if (!drag.mode) {
      if (Math.sqrt(dx * dx + dy * dy) > 20) {
        drag.mode = drag.fixed ? "box" : Math.abs(dx) < 7 ? "horizontal" : Math.abs(dy) < 7 ? "vertical" : "box";
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
      if (drag.fixed) rect = panelShaped(rect, r);
    }
    return { rect: rect, acceptable: rect[2] - rect[0] > MIN_SELECTION || rect[3] - rect[1] > MIN_SELECTION };
  }

  // `rect` grown about its centre to the shape of `panel` ([x, y, w, h]), then
  // moved back inside it (a grown selection is never larger than the panel).
  function panelShaped(rect, panel) {
    var w = rect[2] - rect[0], h = rect[3] - rect[1], ratio = panel[2] / panel[3];
    if (w < h * ratio) w = h * ratio;
    else h = w / ratio;
    var left = clamp((rect[0] + rect[2] - w) / 2, panel[0], panel[0] + panel[2] - w);
    var top = clamp((rect[1] + rect[3] - h) / 2, panel[1], panel[1] + panel[3] - h);
    return [left, top, left + w, top + h];
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
    zoom.drag = {
      ref: ref, from: from, start: currentViews(tile), bounds: tile.spec.panel, mode: null,
      fixed: zoom.plots[ref.plot].fixedAspect,
    };
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
    var b = element("button", "width:24px;height:24px;margin:0;padding:0;display:flex;justify-content:center;" +
      "align-items:center;border:none;border-radius:7px;cursor:pointer;transition:background-color 0.12s;", parent);
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
    var holder = element("div", "display:flex;justify-content:center;align-items:center;margin:0 0 2px 0;line-height:normal;");
    holder.className = "ggrust-toolbar";
    var bar = element("div", "position:relative;display:inline-flex;justify-content:center;align-items:center;" +
      "box-sizing:border-box;height:31px;gap:2px;padding:3px;background-color:" + BAR_BACKGROUND +
      ";border:" + BAR_BORDER + ";border-radius:10px;box-shadow:" + BAR_SHADOW + ";", holder);
    plot.zoom.buttons = TOOLS.map(function (tool) {
      return { name: tool.name, button: button(tool.label, tool.icon, bar, function () { selectTool(plot, tool.name); }) };
    });
    element("span", "width:0.5px;height:14px;margin:0 3px;background-color:rgba(0, 0, 0, 0.18);", bar);
    button("Reset", RESET_ICON, bar, function () {
      if (plot.zoom.tool) selectTool(plot, plot.zoom.tool);
      resetViews(plot);
    });
    if (plot.zoom.plots.some(function (zoomPlot) { return zoomPlot.layers; })) {
      layersMenu(plot, bar);
    }
    plot.root.insertBefore(holder, plot.svg);
  }

  // -- Layer toggles (`ggtb(layers=...)`, a ggrust extension) --
  //
  // Every panel holds one `g.layer` per layer, in layer order (empty when the
  // layer draws nothing). Hiding layer i of a plot hides that group in each of
  // its panels, stops its canvas points from drawing and its tooltip targets
  // from answering. Each plot of a grid has its own list; legends stay.

  var LAYERS_ICON = icon('<path fill-rule="evenodd" d="M8,1.5 1.5,5 8,8.5 14.5,5ZM8,2.64 12.39,5 8,7.36 3.61,5Z"/>' +
    '<path d="M2.24,7.6 1.76,8.48 8,11.84 14.24,8.48 13.76,7.6 8,10.7Z"/>' +
    '<path d="M2.24,10.6 1.76,11.48 8,14.84 14.24,11.48 13.76,10.6 8,13.7Z"/>');

  function layerGroup(tile, index) {
    var panel = childByClass(tile.element, "panel");
    var count = 0;
    for (var child = panel ? panel.firstElementChild : null; child; child = child.nextElementSibling) {
      if (child.getAttribute("class") === "layer" && count++ === index) return child;
    }
    return null;
  }

  function setLayerVisible(plot, zoomPlot, index, visible) {
    zoomPlot.hidden[index] = !visible;
    zoomPlot.tiles.forEach(function (tile) {
      var group = layerGroup(tile, index);
      if (group) {
        group.style.display = visible ? "" : "none";
        var holders = group.querySelectorAll("foreignObject.ggrust-canvas");
        for (var i = 0; i < holders.length; i++) {
          if (holders[i].ggrustCanvas) holders[i].ggrustCanvas.hidden = !visible;
        }
      }
      (plot.layers || []).forEach(function (entry) {
        if (entry.layer === index && samePanel(entry.panel, tile.spec.panel)) entry.hidden = !visible;
      });
      tile.dirty = true; // redraws its canvas points at the current view
    });
    if (plot.overlay) show(plot, null);
    schedule(plot);
  }

  function layersMenu(plot, bar) {
    var menu = element("div", "position:absolute;top:34px;left:50%;transform:translateX(-50%);z-index:2;" +
      "display:none;box-sizing:border-box;min-width:120px;padding:6px 8px;background-color:rgba(255, 255, 255, 0.97);" +
      "border:" + BAR_BORDER + ";border-radius:10px;box-shadow:0 2px 8px rgba(0, 0, 0, 0.12);" +
      "font:12px sans-serif;color:" + BUTTON_SELECTED + ";text-align:left;white-space:nowrap;", bar);
    menu.className = "ggrust-layers";
    var listed = plot.zoom.plots.filter(function (zoomPlot) { return zoomPlot.layers; });
    plot.zoom.plots.forEach(function (zoomPlot, plotIndex) {
      if (!zoomPlot.layers) return;
      if (listed.length > 1) {
        var heading = element("div", "margin:4px 0 2px 0;font-weight:bold;color:" + BUTTON_STROKE + ";", menu);
        heading.textContent = "Plot " + (plotIndex + 1);
      }
      zoomPlot.layers.forEach(function (label, index) {
        var row = element("label", "display:flex;align-items:center;gap:6px;padding:2px 0;cursor:pointer;", menu);
        var box = element("input", "margin:0;cursor:pointer;", row);
        box.type = "checkbox";
        box.checked = !zoomPlot.hidden[index];
        box.addEventListener("change", function () { setLayerVisible(plot, zoomPlot, index, box.checked); });
        element("span", "", row).textContent = label;
      });
    });
    var toggle = button("Layers", LAYERS_ICON, bar, function () {
      var open = menu.style.display === "none";
      menu.style.display = open ? "block" : "none";
      setButton(toggle, open);
    });
    bar.insertBefore(toggle, bar.children[TOOLS.length]); // after the zoom tools, before Reset
    document.addEventListener("mousedown", function (event) {
      if (menu.style.display !== "none" && !bar.contains(event.target)) {
        menu.style.display = "none";
        setButton(toggle, false);
      }
    });
  }

  // Set up zoom/pan when the engine wrote a zoom payload (`ggtb()`).
  function attachZoom(plot) {
    var data = plot.root.querySelector('script[type="application/vnd.ggrust-zoom+json"]');
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
          fixedAspect: !!zoomPlot.fixed_aspect,
          layers: zoomPlot.layers || null,
          hidden: [],
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
    style.textContent = ["panel", "dendrogram-x", "dendrogram-y", "side-x", "side-y", "nested-x", "nested-y"].map(function (name) {
      return "#" + plot.id + " ." + name + " *";
    }).join(", ") + " { vector-effect: non-scaling-stroke; }";
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

  // Point targets that are exactly a canvas batch's markers carry `canvas` (the
  // batch number) instead of x and y: read them from the decoded batch, at the
  // 0.01 px the engine writes inline targets with (same nearest point on ties),
  // as plain arrays (zoomed positions must not round to Float32). False when
  // the batch is missing, so the layer is dropped rather than half indexed.
  function canvasTargets(plot, layer) {
    var t = layer.targets;
    if (t.canvas === undefined) return true;
    var xy = plot.canvasXy && plot.canvasXy[t.canvas];
    var n = t.rows.length;
    if (!xy || xy.length !== 2 * n) return false;
    var canvas = plot.canvasLayers[t.canvas];
    if (!layer.keys && canvas.key && canvas.key.length === n) layer.canvasKeys = canvas;
    t.x = new Array(n);
    t.y = new Array(n);
    for (var i = 0; i < n; i++) {
      t.x[i] = Math.round(xy[2 * i] * 100) / 100;
      t.y[i] = Math.round(xy[2 * i + 1] * 100) / 100;
    }
    return true;
  }

  function attach(plot) {
    var data = plot.root.querySelector('script[type="application/vnd.ggrust-tooltips+json"]');
    if (!data || !plot.svg) {
      return;
    }
    var payload;
    try {
      payload = JSON.parse(data.textContent);
    } catch (error) {
      return;
    }
    plot.layers = (payload.layers || []).filter(function (layer) { return canvasTargets(plot, layer); });
    plot.layers.forEach(function (layer) {
      // Legend keys of the rows: one byte per row, base64.
      if (layer.keys && typeof layer.keys.rows === "string") layer.keys.rows = new Uint8Array(decodeBase64(layer.keys.rows));
    });
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

  // -- Interactive legend --
  //
  // Discrete legend keys (`g.legend-key`) and the marks they stand for
  // (`g.keyed`, runs of marks of one key combination) carry `data-key`: a
  // JSON array of key ids (legend title and label). SVG point batches of mixed
  // keys carry a table of those arrays instead (`data-keys`) and a table index
  // per child element (`data-ki`, base64 bytes), set as `data-k` on each marker
  // at load and hidden by one CSS rule per hidden (batch, index);
  // dense canvas points carry a key index per marker. Clicking a key hides or
  // shows its marks across layers, panels and plots of the figure, and their
  // tooltips; double-clicking isolates it within its legend (again: show all).

  var HIDDEN_KEY_OPACITY = "0.35";

  function keyIds(element) {
    try {
      return JSON.parse(element.getAttribute("data-key")) || [];
    } catch (error) {
      return [];
    }
  }

  function applyLegend(plot) {
    var hidden = plot.legend.hidden;
    plot.legend.keys.forEach(function (key) {
      key.element.style.opacity = hidden[key.id] ? HIDDEN_KEY_OPACITY : "";
    });
    plot.legend.marks.forEach(function (mark) {
      mark.element.style.display = mark.ids.some(function (id) { return hidden[id]; }) ? "none" : "";
    });
    var rules = [];
    plot.legend.batches.forEach(function (batch) {
      batch.combinations.forEach(function (ids, index) {
        if ((ids || []).some(function (id) { return hidden[id]; })) {
          rules.push('g[data-kb="' + batch.id + '"]>[data-k="' + index + '"]');
        }
      });
    });
    if (plot.legend.style) plot.legend.style.textContent = rules.length ? rules.join(",") + "{display:none}" : "";
    plot.layers.forEach(function (layer) {
      if (!layer.keys) return;
      var any = false;
      var flags = layer.keys.combinations.map(function (ids) {
        var off = ids.some(function (id) { return hidden[id]; });
        any = any || off;
        return off;
      });
      layer.hiddenKeys = any ? flags : null;
    });
    (plot.canvases || []).forEach(function (layer) {
      if (!layer.key) return;
      var flags = layer.keys.map(function (ids) {
        return (ids || []).some(function (id) { return hidden[id]; });
      });
      layer.hiddenKeys = flags.some(Boolean) ? flags : null;
      scheduleKeyedDraw(layer);
    });
    if (plot.overlay) show(plot, null);
  }

  // A canvas redraw costs a stamp per marker (about 150 ms for 200k), so a
  // legend click does not draw at once: the key's new opacity paints first,
  // then the canvas is drawn once for every click of that frame (the two
  // clicks and the dblclick of a double-click), and not at all when the
  // hidden keys end as last drawn (the zoom may have drawn them meanwhile).
  function scheduleKeyedDraw(layer) {
    if (layer.keyedDrawPending) return;
    layer.keyedDrawPending = true;
    requestAnimationFrame(function () {
      setTimeout(function () {
        layer.keyedDrawPending = false;
        var view = layer.view;
        if (view && !sameFlags(layer.drawnHiddenKeys, layer.hiddenKeys)) drawCanvas(layer, view[0], view[1], view[2]);
      }, 0);
    });
  }

  function sameFlags(a, b) {
    if (!a || !b) return !a && !b;
    if (a.length !== b.length) return false;
    for (var i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
    return true;
  }

  // Give every marker of a keyed batch its `data-k` (its combination index)
  // from the batch's `data-ki` (base64, one byte per child element), so one
  // CSS rule per hidden (batch, index) hides it. Unkeyed markers get none.
  function setChildKeys(batch, combinations) {
    var text = batch.getAttribute("data-ki");
    if (!text) return;
    var items;
    try {
      items = new Uint8Array(decodeBase64(text));
    } catch (error) {
      return;
    }
    var child = batch.firstElementChild;
    for (var i = 0; child && i < items.length; i++, child = child.nextElementSibling) {
      var ids = combinations[items[i]];
      if (ids && ids.length) child.setAttribute("data-k", items[i]);
    }
  }

  function attachLegend(plot) {
    if (!plot.svg) return;
    var keyElements = plot.svg.querySelectorAll("g.legend-key[data-key]");
    if (!keyElements.length) return;
    plot.legend = { hidden: {}, keys: [], marks: [], batches: [], style: null };
    var marks = plot.svg.querySelectorAll("g.keyed[data-key]");
    for (var m = 0; m < marks.length; m++) {
      plot.legend.marks.push({ element: marks[m], ids: keyIds(marks[m]) });
    }
    // Batch ids are unique per plot (the plot id), so rules never reach another output.
    var batches = plot.svg.querySelectorAll("g[data-keys]");
    for (var b = 0; b < batches.length; b++) {
      var batch = { id: plot.id + "-" + b, combinations: [] };
      try {
        batch.combinations = JSON.parse(batches[b].getAttribute("data-keys")) || [];
      } catch (error) {
        batch.combinations = [];
      }
      batches[b].setAttribute("data-kb", batch.id);
      setChildKeys(batches[b], batch.combinations);
      plot.legend.batches.push(batch);
    }
    if (batches.length) {
      plot.legend.style = document.createElement("style");
      plot.root.appendChild(plot.legend.style);
    }
    var known = {};
    plot.legend.marks.forEach(function (mark) { mark.ids.forEach(function (id) { known[id] = true; }); });
    plot.legend.batches.forEach(function (batch) {
      batch.combinations.forEach(function (ids) { (ids || []).forEach(function (id) { known[id] = true; }); });
    });
    (plot.canvases || []).forEach(function (layer) {
      (layer.keys || []).forEach(function (ids) { (ids || []).forEach(function (id) { known[id] = true; }); });
    });
    for (var k = 0; k < keyElements.length; k++) {
      var element = keyElements[k];
      var id = keyIds(element)[0];
      if (id === undefined || !known[id]) continue; // no marks to hide
      var key = { element: element, id: id, legend: id.split("\u001f")[0] };
      plot.legend.keys.push(key);
      element.style.cursor = "pointer";
      // The whole key cell and its label answer clicks, not just the drawn pixels.
      var box = null;
      try {
        box = element.getBBox();
      } catch (error) {
        box = null;
      }
      if (box && box.width) {
        var hit = document.createElementNS(SVG_NS, "rect");
        ["x", "y", "width", "height"].forEach(function (name) { hit.setAttribute(name, box[name]); });
        hit.setAttribute("fill", "none");
        hit.setAttribute("pointer-events", "all");
        element.insertBefore(hit, element.firstChild);
      }
      element.addEventListener("click", toggleKey.bind(null, plot, key));
      element.addEventListener("dblclick", isolateKey.bind(null, plot, key));
      element.addEventListener("mousedown", function (event) { event.preventDefault(); }); // no text selection
    }
  }

  function toggleKey(plot, key) {
    var hidden = plot.legend.hidden;
    if (hidden[key.id]) delete hidden[key.id];
    else hidden[key.id] = true;
    applyLegend(plot);
  }

  // The two clicks before a double-click toggled the key twice (no change).
  function isolateKey(plot, key) {
    var hidden = plot.legend.hidden;
    var others = plot.legend.keys.filter(function (other) { return other.legend === key.legend && other.id !== key.id; });
    var isolated = !hidden[key.id] && others.every(function (other) { return hidden[other.id]; });
    delete hidden[key.id];
    others.forEach(function (other) {
      if (isolated) delete hidden[other.id];
      else hidden[other.id] = true;
    });
    applyLegend(plot);
  }

  // ---- Animation player -----------------------------------------------------
  // An animation (crates/ggrust-core/src/render/html.rs `to_html_animation`)
  // ships its distinct frames as SVG strings or PNG data URLs, and a timeline:
  // per output frame, the index of the frame it shows. The player plays the
  // timeline at `fps`, looping, under a bar with play/pause, a slider and a
  // frame counter. No tooltips or zoom in animations.
  function attachAnimation(plot) {
    var data = plot.root.querySelector('script[type="application/vnd.ggrust-animation+json"]');
    if (data === null) {
      return false;
    }
    var animation = JSON.parse(data.textContent);
    var holder = plot.root.querySelector(".ggrust-frame");
    var timeline = animation.timeline;
    var count = timeline.length;
    var delay = 1000 / animation.fps;
    var current = 0;
    var shown = timeline[0];
    var timer = null;
    var image = animation.kind === "png" ? holder.querySelector("img") : null;

    var bar = document.createElement("div");
    bar.className = "ggrust-player";
    bar.style.cssText =
      "display:flex;align-items:center;gap:8px;height:32px;line-height:normal;" +
      "font:12px Lato, 'Helvetica Neue', Helvetica, Arial, sans-serif;color:" + TEXT + ";" +
      "box-sizing:border-box;padding:4px 6px;width:" + animation.width + "px;max-width:100%";
    var button = document.createElement("button");
    button.type = "button";
    button.style.cssText =
      "width:26px;height:24px;padding:0;border:1px solid #c8c8c8;border-radius:3px;" +
      "background:#fff;color:" + TEXT + ";cursor:pointer;font-size:12px;line-height:1";
    var slider = document.createElement("input");
    slider.type = "range";
    slider.min = "0";
    slider.max = String(count - 1);
    slider.step = "1";
    slider.value = "0";
    slider.style.cssText = "flex:1;min-width:40px;margin:0";
    slider.setAttribute("aria-label", "frame");
    var counter = document.createElement("span");
    counter.style.cssText = "min-width:64px;text-align:right;font-variant-numeric:tabular-nums";
    bar.appendChild(button);
    bar.appendChild(slider);
    bar.appendChild(counter);
    plot.root.appendChild(bar);

    function show(index) {
      current = index;
      var frame = timeline[index];
      if (frame !== shown) {
        shown = frame;
        if (image !== null) {
          image.src = animation.frames[frame];
        } else {
          holder.innerHTML = animation.frames[frame];
        }
      }
      slider.value = String(index);
      counter.textContent = index + 1 + " / " + count;
    }
    function label() {
      button.textContent = timer === null ? "\u25B6" : "\u275A\u275A";
      button.setAttribute("aria-label", timer === null ? "play" : "pause");
    }
    function tick() {
      show((current + 1) % count);
      timer = window.setTimeout(tick, delay);
    }
    function play() {
      if (timer === null && count > 1) {
        timer = window.setTimeout(tick, delay);
      }
      label();
    }
    function pause() {
      if (timer !== null) {
        window.clearTimeout(timer);
        timer = null;
      }
      label();
    }
    button.addEventListener("click", function () {
      if (timer === null) play();
      else pause();
    });
    slider.addEventListener("input", function () {
      pause();
      show(Number(slider.value));
    });
    plot.animation = { play: play, pause: pause, show: show };
    show(0);
    play();
    return true;
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
    if (attachAnimation(plot)) {
      return plot;
    }
    attachCanvas(plot);
    attach(plot);
    attachZoom(plot);
    attachLegend(plot);
    return plot;
  }

  // `breaks` (the zoom's break generator) and `selection` are exposed for tests.
  window.ggrust = { version: version, mount: mount, lookup: lookup, breaks: flexBreaks, selection: selection };
})
