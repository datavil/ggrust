//! Color space conversions for gradients and palette generation, ported from
//! lets-plot `commons/colorspace` (sRGB D65 <-> XYZ <-> LAB / LUV / HCL) with its
//! constants and rounding, so mapped colors match lets-plot exactly.

use crate::scene::Color;

const REFERENCE_X: f64 = 0.95047 * 100.0;
const REFERENCE_Y: f64 = 100.0;
const REFERENCE_Z: f64 = 1.08883 * 100.0;
const CIE_E: f64 = 0.008856;

/// CIE XYZ (0..100 scale).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Xyz {
    x: f64,
    y: f64,
    z: f64,
}

/// CIE L*a*b*.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    /// Lightness.
    pub l: f64,
    /// Green-red axis.
    pub a: f64,
    /// Blue-yellow axis.
    pub b: f64,
}

/// Polar CIE L*u*v*: hue (degrees), chroma, luminance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hcl {
    /// Hue in degrees, 0..360.
    pub h: f64,
    /// Chroma.
    pub c: f64,
    /// Luminance.
    pub l: f64,
}

fn xyz_from_rgb(color: Color) -> Xyz {
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value > 0.04045 {
            ((value + 0.055) / 1.055).powf(2.4)
        } else {
            value / 12.92
        }
    };
    let (red, green, blue) = (linear(color.red), linear(color.green), linear(color.blue));
    Xyz {
        x: (red * 0.4124 + green * 0.3576 + blue * 0.1805) * 100.0,
        y: (red * 0.2126 + green * 0.7152 + blue * 0.0722) * 100.0,
        z: (red * 0.0193 + green * 0.1192 + blue * 0.9505) * 100.0,
    }
}

/// Out-of-gamut channels are clamped (as lets-plot and R do).
fn rgb_from_xyz(xyz: Xyz) -> Color {
    let gamma = |value: f64| {
        if value > 0.0031308 {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * value
        }
    };
    let (x, y, z) = (xyz.x / 100.0, xyz.y / 100.0, xyz.z / 100.0);
    let channel = |value: f64| (gamma(value) * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::rgb(
        channel(x * 3.2406 + y * -1.5372 + z * -0.4986),
        channel(x * -0.9689 + y * 1.8758 + z * 0.0415),
        channel(x * 0.0557 + y * -0.2040 + z * 1.0570),
    )
}

/// sRGB -> L*a*b*.
pub fn lab_from_rgb(color: Color) -> Lab {
    let xyz = xyz_from_rgb(color);
    let transform = |value: f64| {
        if value > CIE_E {
            value.cbrt()
        } else {
            7.787 * value + 16.0 / 116.0
        }
    };
    let pre_a = transform(xyz.x / REFERENCE_X);
    let pre_l = transform(xyz.y / REFERENCE_Y);
    let pre_b = transform(xyz.z / REFERENCE_Z);
    Lab {
        l: 116.0 * pre_l - 16.0,
        a: 500.0 * (pre_a - pre_l),
        b: 200.0 * (pre_l - pre_b),
    }
}

/// L*a*b* -> sRGB.
pub fn rgb_from_lab(lab: Lab) -> Color {
    let transform = |value: f64| {
        if value.powi(3) > CIE_E {
            value.powi(3)
        } else {
            (value - 16.0 / 116.0) / 7.787
        }
    };
    let pre_y = (lab.l + 16.0) / 116.0;
    let pre_x = lab.a / 500.0 + pre_y;
    let pre_z = pre_y - lab.b / 200.0;
    rgb_from_xyz(Xyz {
        x: transform(pre_x) * REFERENCE_X,
        y: transform(pre_y) * REFERENCE_Y,
        z: transform(pre_z) * REFERENCE_Z,
    })
}

/// u', v' chromaticity of the reference white.
fn reference_uv() -> (f64, f64) {
    let denominator = REFERENCE_X + 15.0 * REFERENCE_Y + 3.0 * REFERENCE_Z;
    (
        4.0 * REFERENCE_X / denominator,
        9.0 * REFERENCE_Y / denominator,
    )
}

/// sRGB -> HCL (polar LUV), lets-plot `luvFromXyz` + `hclFromLuv`.
pub fn hcl_from_rgb(color: Color) -> Hcl {
    let xyz = xyz_from_rgb(color);
    let denominator = xyz.x + 15.0 * xyz.y + 3.0 * xyz.z;
    let (pre_u, pre_v) = if denominator == 0.0 {
        (0.0, 0.0)
    } else {
        (4.0 * xyz.x / denominator, 9.0 * xyz.y / denominator)
    };
    let y = xyz.y / 100.0;
    let pre_l = if y > CIE_E {
        y.cbrt()
    } else {
        7.787 * y + 16.0 / 116.0
    };
    let (reference_u, reference_v) = reference_uv();
    let l = 116.0 * pre_l - 16.0;
    let u = 13.0 * l * (pre_u - reference_u);
    let v = 13.0 * l * (pre_v - reference_v);
    let h = v.atan2(u).to_degrees() % 360.0;
    Hcl {
        h: if h < 0.0 { h + 360.0 } else { h },
        c: (u * u + v * v).sqrt(),
        l: l.clamp(0.0, 100.0),
    }
}

/// HCL (polar LUV) -> sRGB, lets-plot `luvFromHcl` + `xyzFromLuv`.
pub fn rgb_from_hcl(hcl: Hcl) -> Color {
    let radians = hcl.h.to_radians();
    let (luv_u, luv_v) = (hcl.c * radians.cos(), hcl.c * radians.sin());
    if hcl.l == 0.0 {
        return rgb_from_xyz(Xyz {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    }
    let l = (hcl.l + 16.0) / 116.0;
    let pre_y = if l.powi(3) > CIE_E {
        l.powi(3)
    } else {
        (l - 16.0 / 116.0) / 7.787
    };
    let (reference_u, reference_v) = reference_uv();
    let u = luv_u / (13.0 * hcl.l) + reference_u;
    let v = luv_v / (13.0 * hcl.l) + reference_v;
    let y = pre_y * 100.0;
    let x = -(9.0 * y * u) / ((u - 4.0) * v - u * v);
    let z = (9.0 * y - 15.0 * v * y - v * x) / (3.0 * v);
    rgb_from_xyz(Xyz { x, y, z })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_stay_within_one_step() {
        for color in [
            Color::rgb(0x13, 0x2b, 0x43),
            Color::rgb(0x56, 0xb1, 0xf7),
            Color::rgb(0xe4, 0x1a, 0x1c),
            Color::rgb(128, 128, 128),
        ] {
            let back = rgb_from_lab(lab_from_rgb(color));
            let back_hcl = rgb_from_hcl(hcl_from_rgb(color));
            for (original, converted) in [
                (color.red, back.red),
                (color.green, back.green),
                (color.blue, back.blue),
                (color.red, back_hcl.red),
                (color.green, back_hcl.green),
                (color.blue, back_hcl.blue),
            ] {
                assert!(
                    original.abs_diff(converted) <= 1,
                    "{color:?} -> {back:?} / {back_hcl:?}"
                );
            }
        }
    }
}
