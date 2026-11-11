//! `geom_qq`, `geom_qq2` (lets-plot `QQGeom`, `QQ2Geom` are `PointGeom`s) and
//! `geom_qq_line`, `geom_qq2_line` (`QQLineGeom`, `QQ2LineGeom` are `PathGeom`s), each
//! with its own default stat. Drawing is theirs; the quantiles are in `stat/qq.rs`.

use super::{DrawLayer, Geom, PathGeom, PointGeom};
use crate::Result;
use crate::coord::Panel;
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Q-Q points.
pub struct QqPointGeom {
    name: &'static str,
}

/// `geom_qq`: a sample against theoretical quantiles.
pub static QQ_GEOM: QqPointGeom = QqPointGeom { name: "qq" };

/// `geom_qq2`: one sample against another.
pub static QQ2_GEOM: QqPointGeom = QqPointGeom { name: "qq2" };

impl Geom for QqPointGeom {
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        PointGeom.tooltip_behavior()
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        PointGeom.hover_targets(layer, panel)
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn default_stat(&self) -> &'static str {
        self.name
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        PointGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        PointGeom.parameters()
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        PointGeom.draw(layer, panel)
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.legend_defaults()
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PointGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PointGeom.draw_key(styles, cell)
    }
}

/// Q-Q fitting lines.
pub struct QqLineGeom {
    name: &'static str,
}

/// `geom_qq_line`: the fitting line of `geom_qq`.
pub static QQ_LINE_GEOM: QqLineGeom = QqLineGeom { name: "qq_line" };

/// `geom_qq2_line`: the fitting line of `geom_qq2`.
pub static QQ2_LINE_GEOM: QqLineGeom = QqLineGeom { name: "qq2_line" };

impl Geom for QqLineGeom {
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        PathGeom.tooltip_behavior()
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        PathGeom.hover_targets(layer, panel)
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn default_stat(&self) -> &'static str {
        self.name
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        PathGeom.required_aesthetics()
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PathGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        PathGeom.parameters()
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        PathGeom.draw(layer, panel)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PathGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PathGeom.draw_key(styles, cell)
    }
}
