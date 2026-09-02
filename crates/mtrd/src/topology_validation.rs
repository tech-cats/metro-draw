use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{MetroTopology, TopologyStation, topology_layout::Bounds};

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
    if bounds.viewport().is_none()
        || topology
            .stations
            .iter()
            .any(|station| bounds.project(station).is_none())
    {
        return Err(TopologyRenderError::CoordinateRange);
    }

    Ok(())
}

pub(super) fn station_index(
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
}
