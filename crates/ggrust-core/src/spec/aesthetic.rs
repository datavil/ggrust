//! Aesthetic names accepted in `mapping` (and as scale targets).
//!
//! The list mirrors lets-plot's `Aes` plus the `group` mapping option.
//! Aliases `colour` and `col` resolve to `color`, as in lets-plot.
//! Which aesthetics a geom actually supports is checked by the geom, not here.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

macro_rules! aesthetics {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// A visual property that a data column can be mapped to.
        #[allow(missing_docs)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Aesthetic {
            $($variant),+
        }

        impl Aesthetic {
            /// Every aesthetic, in declaration order.
            pub const ALL: &'static [Aesthetic] = &[$(Aesthetic::$variant),+];

            /// The lets-plot option name, e.g. `"xend"`.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Aesthetic::$variant => $name),+
                }
            }
        }

        impl FromStr for Aesthetic {
            type Err = UnknownAesthetic;

            fn from_str(name: &str) -> Result<Self, Self::Err> {
                match name {
                    $($name => Ok(Aesthetic::$variant),)+
                    "colour" | "col" => Ok(Aesthetic::Color),
                    _ => Err(UnknownAesthetic(name.to_owned())),
                }
            }
        }
    };
}

aesthetics! {
    X => "x", Y => "y", Z => "z",
    Color => "color", Fill => "fill",
    PaintA => "paint_a", PaintB => "paint_b", PaintC => "paint_c",
    Alpha => "alpha", Shape => "shape", Linetype => "linetype",
    Size => "size", Stroke => "stroke", Linewidth => "linewidth", Stacksize => "stacksize",
    Width => "width", Height => "height", Binwidth => "binwidth", Violinwidth => "violinwidth",
    Weight => "weight", Intercept => "intercept", Slope => "slope",
    Xintercept => "xintercept", Yintercept => "yintercept",
    Lower => "lower", Middle => "middle", Upper => "upper",
    Xlower => "xlower", Xmiddle => "xmiddle", Xupper => "xupper",
    Sample => "sample", Quantile => "quantile",
    Xmin => "xmin", Xmax => "xmax", Ymin => "ymin", Ymax => "ymax",
    Xend => "xend", Yend => "yend",
    MapId => "map_id", Frame => "frame", Speed => "speed", Flow => "flow",
    Label => "label", Family => "family", Fontface => "fontface",
    Lineheight => "lineheight", Angle => "angle", Hjust => "hjust", Vjust => "vjust",
    Radius => "radius", Slice => "slice", Explode => "explode",
    Istart => "istart", Iend => "iend", Lenstart => "lenstart", Lenend => "lenend",
    SizeStart => "size_start", SizeEnd => "size_end",
    StrokeStart => "stroke_start", StrokeEnd => "stroke_end",
    PointSize => "point_size", PointStroke => "point_stroke",
    SegmentColor => "segment_color", SegmentSize => "segment_size",
    SegmentAlpha => "segment_alpha",
    Group => "group",
}

/// What an aesthetic's values are used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AestheticKind {
    /// A horizontal position (x scale).
    PositionX,
    /// A vertical position (y scale).
    PositionY,
    /// A style with values of this kind.
    Style(crate::style::ValueKind),
    /// Grouping only (`group`).
    Group,
    /// Not drawn by any ggrust geom (map ids, animation frames, ...).
    Other,
}

impl AestheticKind {
    /// The style value kind, for style aesthetics.
    pub fn style(self) -> Option<crate::style::ValueKind> {
        match self {
            AestheticKind::Style(kind) => Some(kind),
            _ => None,
        }
    }
}

impl Aesthetic {
    /// The single source of truth for what this aesthetic's values are.
    pub fn kind(self) -> AestheticKind {
        use crate::style::ValueKind;
        use Aesthetic::*;
        if self.is_x_position() {
            return AestheticKind::PositionX;
        }
        if self.is_y_position() {
            return AestheticKind::PositionY;
        }
        match self {
            Color | Fill | PaintA | PaintB | PaintC | SegmentColor => {
                AestheticKind::Style(ValueKind::Color)
            }
            Shape => AestheticKind::Style(ValueKind::Shape),
            Linetype => AestheticKind::Style(ValueKind::Linetype),
            Label | Family | Fontface => AestheticKind::Style(ValueKind::Text),
            Alpha | Size | Stroke | Linewidth | Stacksize | Width | Height | Binwidth
            | Violinwidth | Weight | Intercept | Slope | Quantile | Lineheight | Angle | Hjust
            | Vjust | Radius | Slice | Explode | SizeStart | SizeEnd | StrokeStart | StrokeEnd
            | PointSize | PointStroke | SegmentSize | SegmentAlpha => {
                AestheticKind::Style(ValueKind::Number)
            }
            Group => AestheticKind::Group,
            _ => AestheticKind::Other,
        }
    }

    /// Whether values of this aesthetic are horizontal positions (trained on the x scale).
    pub fn is_x_position(self) -> bool {
        matches!(
            self,
            Aesthetic::X
                | Aesthetic::Xmin
                | Aesthetic::Xmax
                | Aesthetic::Xend
                | Aesthetic::Xintercept
                | Aesthetic::Xlower
                | Aesthetic::Xmiddle
                | Aesthetic::Xupper
        )
    }

    /// Whether values of this aesthetic are vertical positions (trained on the y scale).
    pub fn is_y_position(self) -> bool {
        matches!(
            self,
            Aesthetic::Y
                | Aesthetic::Ymin
                | Aesthetic::Ymax
                | Aesthetic::Yend
                | Aesthetic::Yintercept
                | Aesthetic::Lower
                | Aesthetic::Middle
                | Aesthetic::Upper
        )
    }
}

/// Error for a name that is not an aesthetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAesthetic(pub String);

impl fmt::Display for UnknownAesthetic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = Aesthetic::ALL.iter().map(|a| a.as_str()).collect();
        write!(
            formatter,
            "unknown aesthetic '{}'; expected one of: {}",
            self.0,
            names.join(", ")
        )
    }
}

impl std::error::Error for UnknownAesthetic {}

impl fmt::Display for Aesthetic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Aesthetic {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        name.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_every_name() {
        for aesthetic in Aesthetic::ALL {
            assert_eq!(aesthetic.as_str().parse::<Aesthetic>(), Ok(*aesthetic));
        }
    }

    #[test]
    fn resolves_color_aliases() {
        assert_eq!("colour".parse::<Aesthetic>(), Ok(Aesthetic::Color));
        assert_eq!("col".parse::<Aesthetic>(), Ok(Aesthetic::Color));
    }

    #[test]
    fn rejects_unknown_name_with_choices() {
        let error = "colr".parse::<Aesthetic>().unwrap_err().to_string();
        assert!(error.starts_with("unknown aesthetic 'colr'; expected one of: x, y, z, color"));
    }
}
