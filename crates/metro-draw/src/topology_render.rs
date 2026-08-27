use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use thiserror::Error;

use crate::{MetroTopology, TopologyPath, TopologyStation};

const SCALE: f64 = 80.0;
const PADDING: f64 = 48.0;
const LABEL_SPACE: f64 = 160.0;
const LINE_WIDTH: f64 = 8.0;
const LANE_GAP: f64 = 3.0;
const TAPER_LENGTH: f64 = 24.0;

/// An error encountered while rendering a metro topology.
#[derive(Debug, Error, PartialEq)]
pub enum TopologyRenderError {
    #[error("station id must not be empty")]
    EmptyStationId,

    #[error("station '{station}' has a non-finite position")]
    NonFinitePosition { station: String },

    #[error("station id '{station}' is defined more than once")]
    DuplicateStation { station: String },

    #[error("line id must not be empty")]
    EmptyLineId,

    #[error("line id '{line}' is defined more than once")]
    DuplicateLine { line: String },

    #[error("line '{line}' refers to unknown station '{station}'")]
    UnknownStation { line: String, station: String },

    #[error("path {path} of line '{line}' must contain at least {minimum} stations")]
    PathTooShort {
        line: String,
        path: usize,
        minimum: usize,
    },

    #[error("path {path} of line '{line}' contains station '{station}' more than once")]
    DuplicateStationInPath {
        line: String,
        path: usize,
        station: String,
    },

    #[error("station coordinates are too large to render")]
    CoordinateRange,
}

/// Render a topology as an SVG topology graph.
///
/// Manifest positions are treated as Cartesian coordinates, so increasing
/// `y` is rendered upwards. Each line path is drawn in its configured color;
/// closed paths are joined back to their first station.
pub fn render_topology_svg(topology: &MetroTopology) -> Result<String, TopologyRenderError> {
    validate_topology(topology)?;
    let stations = station_index(topology)?;
    let segment_lanes = segment_lanes(topology);
    let bounds = Bounds::from_topology(topology);
    let width = (bounds.max_x - bounds.min_x) * SCALE + PADDING * 2.0 + LABEL_SPACE;
    let height = (bounds.max_y - bounds.min_y) * SCALE + PADDING * 2.0;

    if !width.is_finite() || !height.is_finite() {
        return Err(TopologyRenderError::CoordinateRange);
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

    for (line_index, line) in topology.lines.iter().enumerate() {
        for path in &line.paths {
            if let [station_id] = path.stations.as_slice() {
                let station = stations.get(station_id.as_str()).ok_or_else(|| {
                    TopologyRenderError::UnknownStation {
                        line: line.id.clone(),
                        station: station_id.clone(),
                    }
                })?;
                let (x, y) = bounds.project(station)?;
                writeln!(
                    svg,
                    "    <path data-line-id=\"{}\" d=\"M{} {}\" stroke=\"{}\" stroke-width=\"{}\" />",
                    xml_escape(&line.id),
                    number(x),
                    number(y),
                    xml_escape(&line.color),
                    number(LINE_WIDTH),
                )
                .unwrap();
                continue;
            }

            for (start_id, end_id) in path_segments(path) {
                let start =
                    stations
                        .get(start_id)
                        .ok_or_else(|| TopologyRenderError::UnknownStation {
                            line: line.id.clone(),
                            station: start_id.to_owned(),
                        })?;
                let end =
                    stations
                        .get(end_id)
                        .ok_or_else(|| TopologyRenderError::UnknownStation {
                            line: line.id.clone(),
                            station: end_id.to_owned(),
                        })?;
                let key = SegmentKey::new(start_id, end_id);
                let lanes = &segment_lanes[&key];
                let lane_index = lanes
                    .iter()
                    .position(|candidate| *candidate == line_index)
                    .expect("every rendered segment was indexed");
                let offset =
                    (lane_index as f64 - (lanes.len() as f64 - 1.0) / 2.0) * lane_spacing();
                let data = segment_path(bounds, start, end, key.is_forward(start_id), offset)?;

                writeln!(
                    svg,
                    "    <path data-line-id=\"{}\" d=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />",
                    xml_escape(&line.id),
                    data,
                    xml_escape(&line.color),
                    number(LINE_WIDTH),
                )
                .unwrap();
            }
        }
    }
    writeln!(svg, "  </g>").unwrap();
    writeln!(svg, "  <g font-family=\"sans-serif\" font-size=\"14\">").unwrap();

    for station in &topology.stations {
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

/// Validate all invariants required to render a topology graph.
pub fn validate_topology(topology: &MetroTopology) -> Result<(), TopologyRenderError> {
    let stations = station_index(topology)?;
    let mut line_ids = HashSet::with_capacity(topology.lines.len());

    for line in &topology.lines {
        if line.id.trim().is_empty() {
            return Err(TopologyRenderError::EmptyLineId);
        }
        if !line_ids.insert(line.id.as_str()) {
            return Err(TopologyRenderError::DuplicateLine {
                line: line.id.clone(),
            });
        }

        for (path_index, path) in line.paths.iter().enumerate() {
            let minimum = if path.closed { 3 } else { 2 };
            if path.stations.len() < minimum {
                return Err(TopologyRenderError::PathTooShort {
                    line: line.id.clone(),
                    path: path_index + 1,
                    minimum,
                });
            }

            let mut path_stations = HashSet::with_capacity(path.stations.len());
            for station in &path.stations {
                if !stations.contains_key(station.as_str()) {
                    return Err(TopologyRenderError::UnknownStation {
                        line: line.id.clone(),
                        station: station.clone(),
                    });
                }
                if !path_stations.insert(station.as_str()) {
                    return Err(TopologyRenderError::DuplicateStationInPath {
                        line: line.id.clone(),
                        path: path_index + 1,
                        station: station.clone(),
                    });
                }
            }
        }
    }

    let bounds = Bounds::from_topology(topology);
    let width = (bounds.max_x - bounds.min_x) * SCALE + PADDING * 2.0 + LABEL_SPACE;
    let height = (bounds.max_y - bounds.min_y) * SCALE + PADDING * 2.0;
    if !width.is_finite() || !height.is_finite() {
        return Err(TopologyRenderError::CoordinateRange);
    }
    for station in &topology.stations {
        bounds.project(station)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SegmentKey<'a> {
    first: &'a str,
    second: &'a str,
}

impl<'a> SegmentKey<'a> {
    fn new(first: &'a str, second: &'a str) -> Self {
        if first <= second {
            Self { first, second }
        } else {
            Self {
                first: second,
                second: first,
            }
        }
    }

    fn is_forward(self, start: &str) -> bool {
        self.first == start
    }
}

fn segment_lanes(topology: &MetroTopology) -> HashMap<SegmentKey<'_>, Vec<usize>> {
    let mut segments = HashMap::<_, Vec<_>>::new();
    for (line_index, line) in topology.lines.iter().enumerate() {
        for path in &line.paths {
            for (start, end) in path_segments(path) {
                let lanes = segments.entry(SegmentKey::new(start, end)).or_default();
                if !lanes.contains(&line_index) {
                    lanes.push(line_index);
                }
            }
        }
    }
    segments
}

fn path_segments(path: &TopologyPath) -> impl Iterator<Item = (&str, &str)> {
    let adjacent = path
        .stations
        .windows(2)
        .map(|stations| (stations[0].as_str(), stations[1].as_str()));
    let closing = (path.closed && path.stations.len() > 1).then(|| {
        (
            path.stations.last().unwrap().as_str(),
            path.stations.first().unwrap().as_str(),
        )
    });
    adjacent.chain(closing)
}

fn lane_spacing() -> f64 {
    LINE_WIDTH + LANE_GAP
}

fn segment_path(
    bounds: Bounds,
    start: &TopologyStation,
    end: &TopologyStation,
    canonical_direction: bool,
    offset: f64,
) -> Result<String, TopologyRenderError> {
    let (start_x, start_y) = bounds.project(start)?;
    let (end_x, end_y) = bounds.project(end)?;
    let dx = end_x - start_x;
    let dy = end_y - start_y;
    let length = dx.hypot(dy);

    if offset == 0.0 || length == 0.0 {
        return Ok(format!(
            "M{} {} L{} {}",
            number(start_x),
            number(start_y),
            number(end_x),
            number(end_y)
        ));
    }

    let direction_x = dx / length;
    let direction_y = dy / length;
    let canonical_sign = if canonical_direction { 1.0 } else { -1.0 };
    let normal_x = -direction_y * canonical_sign;
    let normal_y = direction_x * canonical_sign;
    let taper = TAPER_LENGTH.min(length / 4.0);
    let first_x = start_x + direction_x * taper + normal_x * offset;
    let first_y = start_y + direction_y * taper + normal_y * offset;
    let second_x = end_x - direction_x * taper + normal_x * offset;
    let second_y = end_y - direction_y * taper + normal_y * offset;

    Ok(format!(
        "M{} {} L{} {} L{} {} L{} {}",
        number(start_x),
        number(start_y),
        number(first_x),
        number(first_y),
        number(second_x),
        number(second_y),
        number(end_x),
        number(end_y)
    ))
}

fn station_index(
    topology: &MetroTopology,
) -> Result<HashMap<&str, &TopologyStation>, TopologyRenderError> {
    let mut stations = HashMap::with_capacity(topology.stations.len());
    for station in &topology.stations {
        if station.id.trim().is_empty() {
            return Err(TopologyRenderError::EmptyStationId);
        }
        if !station.position.x.is_finite() || !station.position.y.is_finite() {
            return Err(TopologyRenderError::NonFinitePosition {
                station: station.id.clone(),
            });
        }
        if stations.insert(station.id.as_str(), station).is_some() {
            return Err(TopologyRenderError::DuplicateStation {
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
    fn from_topology(topology: &MetroTopology) -> Self {
        let Some(first) = topology.stations.first() else {
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
        for station in &topology.stations[1..] {
            bounds.min_x = bounds.min_x.min(station.position.x);
            bounds.max_x = bounds.max_x.max(station.position.x);
            bounds.min_y = bounds.min_y.min(station.position.y);
            bounds.max_y = bounds.max_y.max(station.position.y);
        }
        bounds
    }

    fn project(self, station: &TopologyStation) -> Result<(f64, f64), TopologyRenderError> {
        let x = (station.position.x - self.min_x) * SCALE + PADDING;
        let y = (self.max_y - station.position.y) * SCALE + PADDING;
        if x.is_finite() && y.is_finite() {
            Ok((x, y))
        } else {
            Err(TopologyRenderError::CoordinateRange)
        }
    }
}

fn station_label(station: &TopologyStation) -> &str {
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
    use crate::{TopologyLine, TopologyPath, TopologyPosition};

    fn topology() -> MetroTopology {
        MetroTopology {
            stations: vec![
                TopologyStation {
                    id: "south&west".into(),
                    names: [("en".into(), vec!["South <West>".into()])].into(),
                    position: TopologyPosition { x: -1.0, y: 1.0 },
                },
                TopologyStation {
                    id: "north".into(),
                    names: [("en".into(), vec!["North".into()])].into(),
                    position: TopologyPosition { x: 1.0, y: 3.0 },
                },
            ],
            lines: vec![TopologyLine {
                id: "red\"line".into(),
                names: Default::default(),
                color: "#f00".into(),
                paths: vec![TopologyPath {
                    stations: vec!["south&west".into(), "north".into()],
                    closed: false,
                }],
            }],
        }
    }

    fn horizontal_shared_topology(line_count: usize) -> MetroTopology {
        let stations = vec![
            TopologyStation {
                id: "a".into(),
                names: Default::default(),
                position: TopologyPosition { x: 0.0, y: 0.0 },
            },
            TopologyStation {
                id: "b".into(),
                names: Default::default(),
                position: TopologyPosition { x: 2.0, y: 0.0 },
            },
        ];
        let lines = (0..line_count)
            .map(|index| TopologyLine {
                id: format!("line-{index}"),
                names: Default::default(),
                color: format!("#{index}{index}{index}"),
                paths: vec![TopologyPath {
                    stations: if index == 1 {
                        vec!["b".into(), "a".into()]
                    } else {
                        vec!["a".into(), "b".into()]
                    },
                    closed: false,
                }],
            })
            .collect();
        MetroTopology { stations, lines }
    }

    #[test]
    fn renders_paths_stations_labels_and_cartesian_y_axis() {
        let svg = render_topology_svg(&topology()).unwrap();

        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("data-line-id=\"red&quot;line\""));
        assert!(svg.contains("d=\"M48 208 L208 48\""));
        assert!(svg.contains("data-station-id=\"south&amp;west\""));
        assert!(svg.contains(">South &lt;West&gt;</text>"));
        assert!(svg.ends_with("</svg>\n"));
    }

    #[test]
    fn renders_two_shared_lines_in_separate_lanes() {
        let svg = render_topology_svg(&horizontal_shared_topology(2)).unwrap();

        assert!(svg.contains("data-line-id=\"line-0\" d=\"M48 48 L72 42.5 L184 42.5 L208 48\""));
        assert!(svg.contains("data-line-id=\"line-1\" d=\"M208 48 L184 53.5 L72 53.5 L48 48\""));
    }

    #[test]
    fn renders_three_shared_lines_symmetrically() {
        let svg = render_topology_svg(&horizontal_shared_topology(3)).unwrap();

        assert!(svg.contains("d=\"M48 48 L72 37 L184 37 L208 48\""));
        assert!(svg.contains("d=\"M208 48 L48 48\""));
        assert!(svg.contains("d=\"M48 48 L72 59 L184 59 L208 48\""));
    }

    #[test]
    fn indexes_closing_segments_and_deduplicates_a_line_lane() {
        let mut topology = horizontal_shared_topology(1);
        topology.stations.push(TopologyStation {
            id: "c".into(),
            names: Default::default(),
            position: TopologyPosition { x: 1.0, y: 1.0 },
        });
        topology.lines[0].paths = vec![
            TopologyPath {
                stations: vec!["a".into(), "b".into(), "c".into()],
                closed: true,
            },
            TopologyPath {
                stations: vec!["a".into(), "b".into()],
                closed: false,
            },
        ];

        let lanes = segment_lanes(&topology);
        assert_eq!(lanes[&SegmentKey::new("a", "c")], vec![0]);
        assert_eq!(lanes[&SegmentKey::new("a", "b")], vec![0]);
    }

    #[test]
    fn reports_unknown_stations() {
        let mut topology = topology();
        topology.lines[0].paths[0].stations.push("missing".into());

        assert_eq!(
            render_topology_svg(&topology),
            Err(TopologyRenderError::UnknownStation {
                line: "red\"line".into(),
                station: "missing".into(),
            })
        );
    }

    #[test]
    fn validates_ids_station_references_and_coordinates() {
        let mut invalid = topology();
        invalid.stations[0].id.clear();
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::EmptyStationId)
        );

        let mut invalid = topology();
        invalid.stations[1].id = invalid.stations[0].id.clone();
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::DuplicateStation {
                station: "south&west".into()
            })
        );

        let mut invalid = topology();
        invalid.stations[0].position.x = f64::NAN;
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::NonFinitePosition {
                station: "south&west".into()
            })
        );

        let mut invalid = topology();
        invalid.stations[0].position.x = -f64::MAX;
        invalid.stations[1].position.x = f64::MAX;
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::CoordinateRange)
        );

        let mut invalid = topology();
        invalid.lines[0].id.clear();
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::EmptyLineId)
        );

        let mut invalid = topology();
        invalid.lines.push(invalid.lines[0].clone());
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::DuplicateLine {
                line: "red\"line".into()
            })
        );

        let mut invalid = topology();
        invalid.lines[0].paths[0].stations[1] = "missing".into();
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::UnknownStation {
                line: "red\"line".into(),
                station: "missing".into()
            })
        );
    }

    #[test]
    fn validates_path_lengths_and_repeated_stations() {
        let mut invalid = topology();
        invalid.lines[0].paths[0].stations.pop();
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::PathTooShort {
                line: "red\"line".into(),
                path: 1,
                minimum: 2,
            })
        );

        let mut invalid = topology();
        invalid.lines[0].paths[0].closed = true;
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::PathTooShort {
                line: "red\"line".into(),
                path: 1,
                minimum: 3,
            })
        );

        let mut invalid = topology();
        invalid.lines[0].paths[0].stations.push("south&west".into());
        assert_eq!(
            validate_topology(&invalid),
            Err(TopologyRenderError::DuplicateStationInPath {
                line: "red\"line".into(),
                path: 1,
                station: "south&west".into(),
            })
        );
    }

    #[test]
    fn renders_an_empty_topology() {
        let svg = render_topology_svg(&MetroTopology {
            stations: vec![],
            lines: vec![],
        })
        .unwrap();

        assert!(svg.contains("viewBox=\"0 0 256 96\""));
    }
}
