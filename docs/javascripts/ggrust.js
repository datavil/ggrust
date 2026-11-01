// Scales each live plot frame down to fit its column on narrow screens.
// CSS `zoom` (unlike a transform) also shrinks the layout box, and pointer
// events inside the frame stay aligned, so tooltips keep working.
(function () {
  function fit() {
    document.querySelectorAll(".ggrust-figure iframe").forEach(function (frame) {
      var available = frame.parentElement.clientWidth;
      var needed = Number(frame.getAttribute("width")) + 2; // + border
      frame.style.zoom = available > 0 && available < needed ? available / needed : "";
    });
  }
  // `document$` fires on every page load, including instant navigation.
  if (window.document$) {
    window.document$.subscribe(fit);
  } else {
    document.addEventListener("DOMContentLoaded", fit);
  }
  window.addEventListener("resize", fit);
})();
