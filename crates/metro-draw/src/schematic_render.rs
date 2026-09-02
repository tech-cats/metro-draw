use std::fmt::Write;

use crate::{
    OctilinearAxis, SchematicManifest, SchematicStrokeAlignment,
    schematic_validation::{
        Point, PreparedPath, PreparedPointKind, PreparedSchematic, PreparedShape,
        SchematicRenderError, axis_vector, corner_tangents, prepare_schematic,
    },
};

const PADDING: f64 = 24.0;

/// Render a semantic schematic manifest as an SVG schematic map.
///
/// Coordinates use SVG orientation directly: x increases to the right and y
/// increases downwards. The manifest is semantically resolved and its derived
/// geometry is validated before any SVG is generated.
pub fn render_schematic_svg(schematic: &SchematicManifest) -> Result<String, SchematicRenderError> {
    let prepared = prepare_schematic(schematic)?;
    let bounds = Bounds::from_schematic(&prepared)?;
    let mut svg = String::new();

    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="{}" height="{}" role="img">"#,
        number(bounds.min_x),
        number(bounds.min_y),
        number(bounds.width()),
        number(bounds.height()),
        number(bounds.width()),
        number(bounds.height()),
    )
    .unwrap();
    writeln!(svg, "  <title>Metro schematic map</title>").unwrap();
    writeln!(
        svg,
        "  <g fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">"
    )
    .unwrap();
    for stroke in &prepared.strokes {
        for path in &stroke.paths {
            let data = path_data(path);
            writeln!(
                svg,
                "    <path data-line-id=\"{}\" d=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
                xml_escape(stroke.id),
                data,
                xml_escape(stroke.color),
                number(prepared.line_width),
            )
            .unwrap();
        }
    }
    writeln!(svg, "  </g>").unwrap();
    writeln!(svg, "  <g>").unwrap();
    for symbol in &prepared.symbols {
        writeln!(
            svg,
            "    <g data-station-id=\"{}\">",
            xml_escape(&symbol.station.id)
        )
        .unwrap();
        match symbol.shape {
            PreparedShape::Circle { diameter } => {
                writeln!(
                    svg,
                    "      <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" />",
                    number(symbol.center.x),
                    number(symbol.center.y),
                    number(diameter / 2.0),
                    xml_escape(symbol.fill),
                )
                .unwrap();
                let outline_diameter =
                    aligned_size(diameter, symbol.stroke_width, symbol.stroke_alignment);
                writeln!(
                    svg,
                    "      <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" />",
                    number(symbol.center.x),
                    number(symbol.center.y),
                    number(outline_diameter / 2.0),
                    xml_escape(symbol.stroke),
                    number(symbol.stroke_width),
                )
                .unwrap();
            }
            PreparedShape::Capsule {
                axis,
                diameter,
                length,
            } => {
                write_capsule(
                    &mut svg,
                    symbol.center,
                    axis,
                    length,
                    diameter,
                    symbol.fill,
                    None,
                );
                let outline_length =
                    aligned_size(length, symbol.stroke_width, symbol.stroke_alignment);
                let outline_diameter =
                    aligned_size(diameter, symbol.stroke_width, symbol.stroke_alignment);
                write_capsule(
                    &mut svg,
                    symbol.center,
                    axis,
                    outline_length,
                    outline_diameter,
                    "none",
                    Some((symbol.stroke, symbol.stroke_width)),
                );
            }
        }
        writeln!(svg, "    </g>").unwrap();
    }
    writeln!(svg, "  </g>").unwrap();
    writeln!(svg, "</svg>").unwrap();
    Ok(svg)
}

pub(super) fn validate_render_bounds(
    schematic: &PreparedSchematic<'_>,
) -> Result<(), SchematicRenderError> {
    Bounds::from_schematic(schematic).map(|_| ())
}

fn write_capsule(
    svg: &mut String,
    center: Point,
    axis: OctilinearAxis,
    length: f64,
    diameter: f64,
    fill: &str,
    stroke: Option<(&str, f64)>,
) {
    let angle = axis_angle(axis);
    let stroke_attributes = stroke.map_or_else(String::new, |(color, width)| {
        format!(
            " stroke=\"{}\" stroke-width=\"{}\"",
            xml_escape(color),
            number(width)
        )
    });
    writeln!(
        svg,
        "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"{} transform=\"translate({} {}) rotate({})\" />",
        number(-length / 2.0),
        number(-diameter / 2.0),
        number(length),
        number(diameter),
        number(diameter / 2.0),
        xml_escape(fill),
        stroke_attributes,
        number(center.x),
        number(center.y),
        number(angle),
    )
    .unwrap();
}

fn path_data(path: &PreparedPath<'_>) -> String {
    if path.closed {
        closed_path_data(path)
    } else {
        open_path_data(path)
    }
}

fn open_path_data(path: &PreparedPath<'_>) -> String {
    let mut data = format!(
        "M{} {}",
        number(path.points[0].position.x),
        number(path.points[0].position.y)
    );
    for index in 1..path.points.len() {
        let point = path.points[index];
        if let PreparedPointKind::Corner { radius, .. } = point.kind {
            let (entry, exit, sweep) = corner_tangents(
                path.points[index - 1].position,
                point.position,
                path.points[index + 1].position,
                radius,
            );
            write!(
                data,
                " L{} {} A{} {} 0 0 {} {} {}",
                number(entry.x),
                number(entry.y),
                number(radius),
                number(radius),
                u8::from(sweep),
                number(exit.x),
                number(exit.y),
            )
            .unwrap();
        } else {
            write!(
                data,
                " L{} {}",
                number(point.position.x),
                number(point.position.y)
            )
            .unwrap();
        }
    }
    data
}

fn closed_path_data(path: &PreparedPath<'_>) -> String {
    let first = path.points[0];
    let start = match first.kind {
        PreparedPointKind::Corner { radius, .. } => {
            corner_tangents(
                path.points[path.points.len() - 1].position,
                first.position,
                path.points[1].position,
                radius,
            )
            .1
        }
        PreparedPointKind::Anchor { .. } => first.position,
    };
    let mut data = format!("M{} {}", number(start.x), number(start.y));
    for offset in 1..=path.points.len() {
        let index = offset % path.points.len();
        let point = path.points[index];
        match point.kind {
            PreparedPointKind::Corner { radius, .. } => {
                let previous = path.points[(index + path.points.len() - 1) % path.points.len()];
                let next = path.points[(index + 1) % path.points.len()];
                let (entry, exit, sweep) =
                    corner_tangents(previous.position, point.position, next.position, radius);
                write!(
                    data,
                    " L{} {} A{} {} 0 0 {} {} {}",
                    number(entry.x),
                    number(entry.y),
                    number(radius),
                    number(radius),
                    u8::from(sweep),
                    number(exit.x),
                    number(exit.y),
                )
                .unwrap();
            }
            PreparedPointKind::Anchor { .. } => {
                write!(
                    data,
                    " L{} {}",
                    number(point.position.x),
                    number(point.position.y)
                )
                .unwrap();
            }
        }
    }
    data.push_str(" Z");
    data
}

fn aligned_size(size: f64, stroke_width: f64, alignment: SchematicStrokeAlignment) -> f64 {
    match alignment {
        SchematicStrokeAlignment::Inside => (size - stroke_width).max(0.0),
        SchematicStrokeAlignment::Center => size,
        SchematicStrokeAlignment::Outside => size + stroke_width,
    }
}

fn axis_angle(axis: OctilinearAxis) -> f64 {
    match axis {
        OctilinearAxis::Horizontal => 0.0,
        OctilinearAxis::FallingDiagonal => 45.0,
        OctilinearAxis::Vertical => 90.0,
        OctilinearAxis::RisingDiagonal => -45.0,
    }
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn from_schematic(schematic: &PreparedSchematic<'_>) -> Result<Self, SchematicRenderError> {
        let mut bounds: Option<Self> = None;
        for stroke in &schematic.strokes {
            for path in &stroke.paths {
                for point in &path.points {
                    extend_point(&mut bounds, point.position, schematic.line_width / 2.0);
                }
            }
        }
        for symbol in &schematic.symbols {
            let stroke_outset = match symbol.stroke_alignment {
                SchematicStrokeAlignment::Inside => 0.0,
                SchematicStrokeAlignment::Center => symbol.stroke_width / 2.0,
                SchematicStrokeAlignment::Outside => symbol.stroke_width,
            };
            let (half_x, half_y) = match symbol.shape {
                PreparedShape::Circle { diameter } => {
                    let radius = diameter / 2.0 + stroke_outset;
                    (radius, radius)
                }
                PreparedShape::Capsule {
                    axis,
                    diameter,
                    length,
                } => {
                    let (unit_x, unit_y) = axis_vector(axis);
                    let half_length = length / 2.0 + stroke_outset;
                    let half_diameter = diameter / 2.0 + stroke_outset;
                    (
                        unit_x.abs() * half_length + unit_y.abs() * half_diameter,
                        unit_y.abs() * half_length + unit_x.abs() * half_diameter,
                    )
                }
            };
            extend_rect(
                &mut bounds,
                symbol.center.x - half_x,
                symbol.center.y - half_y,
                symbol.center.x + half_x,
                symbol.center.y + half_y,
            );
        }
        let Some(bounds) = bounds else {
            return Ok(Self {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 256.0,
                max_y: 96.0,
            });
        };
        let bounds = Self {
            min_x: bounds.min_x - PADDING,
            min_y: bounds.min_y - PADDING,
            max_x: bounds.max_x + PADDING,
            max_y: bounds.max_y + PADDING,
        };
        if [bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y]
            .into_iter()
            .all(f64::is_finite)
            && bounds.width().is_finite()
            && bounds.height().is_finite()
        {
            Ok(bounds)
        } else {
            Err(SchematicRenderError::CoordinateRange)
        }
    }

    fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    fn height(self) -> f64 {
        self.max_y - self.min_y
    }
}

fn extend_point(bounds: &mut Option<Bounds>, point: Point, outset: f64) {
    extend_rect(
        bounds,
        point.x - outset,
        point.y - outset,
        point.x + outset,
        point.y + outset,
    );
}

fn extend_rect(bounds: &mut Option<Bounds>, min_x: f64, min_y: f64, max_x: f64, max_y: f64) {
    match bounds {
        Some(bounds) => {
            bounds.min_x = bounds.min_x.min(min_x);
            bounds.min_y = bounds.min_y.min(min_y);
            bounds.max_x = bounds.max_x.max(max_x);
            bounds.max_y = bounds.max_y.max(max_y);
        }
        None => {
            *bounds = Some(Bounds {
                min_x,
                min_y,
                max_x,
                max_y,
            });
        }
    }
}

fn number(value: f64) -> String {
    let value = if value == 0.0 { 0.0 } else { value };
    let formatted = format!("{value:.3}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r##"
options:
  lines: { width: 8.0 }
  stations:
    common:
      fill: { diameter: 18.0, color: { type: unified, value: "#fff" } }
      stroke: { width: 2.0, alignment: center, color: { type: follow-line } }
    interchange:
      fill: { width: 18.0, color: "#fff" }
      stroke: { width: 2.0, alignment: outside, color: "#000" }
stations:
  - id: west
    position: [0.0, 40.0]
    names: { en: [West] }
    symbol: { type: circle }
  - id: central
    position: [80.0, 80.0]
    names: { en: [Central] }
    symbol: { type: capsule, axis: rising_diagonal, anchor_count: 1, anchor_interval: 24.0 }
corners:
  - { id: bend, position: [40.0, 40.0], radius: 8.0 }
lines:
  - id: red&line
    names: { en: [Red] }
    color: "#f00"
    paths:
      - closed: false
        visits:
          - { type: station, station_id: west, port: { type: single_line } }
          - { type: corner, corner_id: bend }
          - { type: station, station_id: central, port: { type: interchange, interchange: { type: single_perpendicular } } }
"##;

    #[test]
    fn renders_rounded_paths_and_station_symbols() {
        let schematic = SchematicManifest::from_yaml(YAML).unwrap();
        let svg = render_schematic_svg(&schematic).unwrap();

        assert!(svg.contains("<title>Metro schematic map</title>"));
        assert!(svg.contains("data-line-id=\"red&amp;line\""));
        assert!(svg.contains("d=\"M0 40 L36.686 40 A8 8 0 0 1 42.343 42.343 L80 80\""));
        assert!(svg.contains("data-station-id=\"west\""));
        assert!(svg.contains("rotate(-45)"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn rejects_non_octilinear_geometry() {
        let yaml = YAML.replace("position: [40.0, 40.0]", "position: [40.0, 50.0]");
        let schematic = SchematicManifest::from_yaml(&yaml).unwrap();
        assert!(matches!(
            render_schematic_svg(&schematic),
            Err(SchematicRenderError::NonOctilinearLeg { .. })
        ));
    }
}
