use std::collections::HashMap;
use std::fmt::Write;

use crate::{
    MetroTopology, TopologyPath, TopologyStation,
    topology_layout::Bounds,
    topology_validation::{TopologyRenderError, station_index, validate_topology},
};

const LINE_WIDTH: f64 = 8.0;
const LANE_GAP: f64 = 3.0;
const TAPER_LENGTH: f64 = 24.0;

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
    let (width, height) = bounds
        .viewport()
        .ok_or(TopologyRenderError::CoordinateRange)?;

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
                let (x, y) = project(bounds, station)?;
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
        let (x, y) = project(bounds, station)?;
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
    let (start_x, start_y) = project(bounds, start)?;
    let (end_x, end_y) = project(bounds, end)?;
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

fn project(bounds: Bounds, station: &TopologyStation) -> Result<(f64, f64), TopologyRenderError> {
    bounds
        .project(station)
        .ok_or(TopologyRenderError::CoordinateRange)
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
    fn renders_an_empty_topology() {
        let svg = render_topology_svg(&MetroTopology {
            stations: vec![],
            lines: vec![],
        })
        .unwrap();

        assert!(svg.contains("viewBox=\"0 0 256 96\""));
    }
}
