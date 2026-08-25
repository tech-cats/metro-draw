use std::collections::HashMap;
use std::fmt::Write;

use thiserror::Error;

use crate::{MetroMap, Station};

const SCALE: f64 = 80.0;
const PADDING: f64 = 48.0;
const LABEL_SPACE: f64 = 160.0;

/// An error encountered while rendering a metro map.
#[derive(Debug, Error, PartialEq)]
pub enum RenderError {
    #[error("station '{station}' has a non-finite position")]
    NonFinitePosition { station: String },

    #[error("station id '{station}' is defined more than once")]
    DuplicateStation { station: String },

    #[error("line '{line}' refers to unknown station '{station}'")]
    UnknownStation { line: String, station: String },

    #[error("station coordinates are too large to render")]
    CoordinateRange,
}

/// Render a map as an SVG topology graph.
///
/// Manifest positions are treated as Cartesian coordinates, so increasing
/// `y` is rendered upwards. Each line path is drawn in its configured color;
/// closed paths are joined back to their first station.
pub fn render_topology_svg(map: &MetroMap) -> Result<String, RenderError> {
    let stations = station_index(map)?;
    let bounds = Bounds::from_map(map);
    let width = (bounds.max_x - bounds.min_x) * SCALE + PADDING * 2.0 + LABEL_SPACE;
    let height = (bounds.max_y - bounds.min_y) * SCALE + PADDING * 2.0;

    if !width.is_finite() || !height.is_finite() {
        return Err(RenderError::CoordinateRange);
    }

    let mut svg = String::new();
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}" role="img">"#,
        number(width),
        number(height),
        number(width),
        number(height),
    )
    .unwrap();
    writeln!(svg, "  <title>Metro topology map</title>").unwrap();
    writeln!(
        svg,
        "  <g fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">"
    )
    .unwrap();

    for line in &map.lines {
        for path in &line.paths {
            if path.stations.is_empty() {
                continue;
            }

            let mut data = String::new();
            for (index, station_id) in path.stations.iter().enumerate() {
                let station = stations.get(station_id.as_str()).ok_or_else(|| {
                    RenderError::UnknownStation {
                        line: line.id.clone(),
                        station: station_id.clone(),
                    }
                })?;
                let (x, y) = bounds.project(station)?;
                write!(
                    data,
                    "{}{} {}",
                    if index == 0 { "M" } else { " L" },
                    number(x),
                    number(y)
                )
                .unwrap();
            }
            if path.closed && path.stations.len() > 1 {
                data.push_str(" Z");
            }

            writeln!(
                svg,
                "    <path data-line-id=\"{}\" d=\"{}\" stroke=\"{}\" stroke-width=\"8\" />",
                xml_escape(&line.id),
                data,
                xml_escape(&line.color),
            )
            .unwrap();
        }
    }
    writeln!(svg, "  </g>").unwrap();
    writeln!(svg, "  <g font-family=\"sans-serif\" font-size=\"14\">").unwrap();

    for station in &map.stations {
        let (x, y) = bounds.project(station)?;
        let label = station_label(station);
        writeln!(
            svg,
            "    <g data-station-id=\"{}\"><circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"white\" stroke=\"#222\" stroke-width=\"3\" /><text x=\"{}\" y=\"{}\" dominant-baseline=\"middle\">{}</text></g>",
            xml_escape(&station.id),
            number(x),
            number(y),
            number(x + 13.0),
            number(y),
            xml_escape(label),
        )
        .unwrap();
    }

    writeln!(svg, "  </g>").unwrap();
    writeln!(svg, "</svg>").unwrap();
    Ok(svg)
}

fn station_index(map: &MetroMap) -> Result<HashMap<&str, &Station>, RenderError> {
    let mut stations = HashMap::with_capacity(map.stations.len());
    for station in &map.stations {
        if !station.position.x.is_finite() || !station.position.y.is_finite() {
            return Err(RenderError::NonFinitePosition {
                station: station.id.clone(),
            });
        }
        if stations.insert(station.id.as_str(), station).is_some() {
            return Err(RenderError::DuplicateStation {
                station: station.id.clone(),
            });
        }
    }
    Ok(stations)
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Bounds {
    fn from_map(map: &MetroMap) -> Self {
        let Some(first) = map.stations.first() else {
            return Self {
                min_x: 0.0,
                max_x: 0.0,
                min_y: 0.0,
                max_y: 0.0,
            };
        };

        let mut bounds = Self {
            min_x: first.position.x,
            max_x: first.position.x,
            min_y: first.position.y,
            max_y: first.position.y,
        };
        for station in &map.stations[1..] {
            bounds.min_x = bounds.min_x.min(station.position.x);
            bounds.max_x = bounds.max_x.max(station.position.x);
            bounds.min_y = bounds.min_y.min(station.position.y);
            bounds.max_y = bounds.max_y.max(station.position.y);
        }
        bounds
    }

    fn project(self, station: &Station) -> Result<(f64, f64), RenderError> {
        let x = (station.position.x - self.min_x) * SCALE + PADDING;
        let y = (self.max_y - station.position.y) * SCALE + PADDING;
        if x.is_finite() && y.is_finite() {
            Ok((x, y))
        } else {
            Err(RenderError::CoordinateRange)
        }
    }
}

fn station_label(station: &Station) -> &str {
    station
        .names
        .get("en")
        .and_then(|names| names.first())
        .or_else(|| station.names.values().find_map(|names| names.first()))
        .map(String::as_str)
        .unwrap_or(&station.id)
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
    use crate::{Line, LinePath, Position};

    fn map() -> MetroMap {
        MetroMap {
            stations: vec![
                Station {
                    id: "south&west".into(),
                    names: [("en".into(), vec!["South <West>".into()])].into(),
                    position: Position { x: -1.0, y: 1.0 },
                },
                Station {
                    id: "north".into(),
                    names: [("en".into(), vec!["North".into()])].into(),
                    position: Position { x: 1.0, y: 3.0 },
                },
            ],
            lines: vec![Line {
                id: "red\"line".into(),
                names: Default::default(),
                color: "#f00".into(),
                paths: vec![LinePath {
                    stations: vec!["south&west".into(), "north".into()],
                    closed: true,
                }],
            }],
        }
    }

    #[test]
    fn renders_paths_stations_labels_and_cartesian_y_axis() {
        let svg = render_topology_svg(&map()).unwrap();

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("data-line-id=\"red&quot;line\""));
        assert!(svg.contains("d=\"M48 208 L208 48 Z\""));
        assert!(svg.contains("data-station-id=\"south&amp;west\""));
        assert!(svg.contains(">South &lt;West&gt;</text>"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn reports_unknown_stations() {
        let mut map = map();
        map.lines[0].paths[0].stations.push("missing".into());

        assert_eq!(
            render_topology_svg(&map),
            Err(RenderError::UnknownStation {
                line: "red\"line".into(),
                station: "missing".into(),
            })
        );
    }

    #[test]
    fn renders_an_empty_map() {
        let svg = render_topology_svg(&MetroMap {
            stations: vec![],
            lines: vec![],
        })
        .unwrap();

        assert!(svg.contains("viewBox=\"0 0 256 96\""));
    }
}
