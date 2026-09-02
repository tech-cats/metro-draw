use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::{
    OctilinearAxis, SchematicInterchangePort, SchematicManifest, SchematicPoint,
    SchematicRouteVisit, SchematicStation, SchematicStationColor, SchematicStationPort,
    SchematicStationSymbol,
};

const EPSILON: f64 = 1.0e-9;

/// An invariant violation that prevents a schematic manifest from rendering.
#[derive(Debug, Error, PartialEq)]
pub enum SchematicRenderError {
    #[error("station id must not be empty")]
    EmptyStationId,
    #[error("station id '{station}' is defined more than once")]
    DuplicateStation { station: String },
    #[error("corner id must not be empty")]
    EmptyCornerId,
    #[error("corner id '{corner}' is defined more than once")]
    DuplicateCorner { corner: String },
    #[error("line id must not be empty")]
    EmptyLineId,
    #[error("line id '{line}' is defined more than once")]
    DuplicateLine { line: String },
    #[error("{kind} '{id}' has an empty locale key")]
    EmptyLocale { kind: &'static str, id: String },
    #[error("{kind} '{id}' locale '{locale}' has no non-empty canonical name")]
    EmptyCanonicalName {
        kind: &'static str,
        id: String,
        locale: String,
    },
    #[error("line '{line}' refers to unknown station '{station}'")]
    UnknownStation { line: String, station: String },
    #[error("line '{line}' refers to unknown corner '{corner}'")]
    UnknownCorner { line: String, corner: String },
    #[error("corner '{corner}' is not referenced by a line")]
    UnreferencedCorner { corner: String },
    #[error("path {path} of line '{line}' must contain at least {minimum} station visits")]
    PathTooShort {
        line: String,
        path: usize,
        minimum: usize,
    },
    #[error("open path {path} of line '{line}' must begin and end at stations")]
    OpenPathEndpoint { line: String, path: usize },
    #[error("path {path} of line '{line}' visits station '{station}' more than once")]
    DuplicateStationInPath {
        line: String,
        path: usize,
        station: String,
    },
    #[error("path {path} of line '{line}' refers to corner '{corner}' more than once")]
    DuplicateCornerInPath {
        line: String,
        path: usize,
        corner: String,
    },
    #[error("station '{station}' uses a port incompatible with its symbol")]
    IncompatiblePort { station: String },
    #[error(
        "station '{station}' uses a perpendicular port incompatible with anchor_count {anchor_count}"
    )]
    IncompatiblePerpendicularPort { station: String, anchor_count: u8 },
    #[error(
        "station '{station}' uses perpendicular anchor {index}, outside anchor_count {anchor_count}"
    )]
    PerpendicularAnchorOutOfRange {
        station: String,
        index: u8,
        anchor_count: u8,
    },
    #[error("station '{station}' cannot use an oblique port with anchor_count {anchor_count}")]
    ObliquePortWithMultipleAnchors { station: String, anchor_count: u8 },
    #[error("station port '{port}' is referenced by lines '{first_line}' and '{second_line}'")]
    PortSharedByLines {
        port: String,
        first_line: String,
        second_line: String,
    },
    #[error("corner '{corner}' is referenced by lines '{first_line}' and '{second_line}'")]
    CornerSharedByLines {
        corner: String,
        first_line: String,
        second_line: String,
    },
    #[error(
        "station '{station}' must reference each of its {anchor_count} perpendicular anchors exactly once"
    )]
    IncompletePerpendicularPorts { station: String, anchor_count: u8 },
    #[error("station '{station}' anchor_interval must be at least the line width")]
    AnchorIntervalTooSmall { station: String },
    #[error("path {path} of line '{line}' has coincident consecutive points")]
    CoincidentPoints { line: String, path: usize },
    #[error("path {path} of line '{line}' contains a non-octilinear leg")]
    NonOctilinearLeg { line: String, path: usize },
    #[error("station '{station}' is used on an axis forbidden by its port")]
    InvalidPortAxis { station: String },
    #[error("path {path} of line '{line}' bends at station '{station}'")]
    BendAtStation {
        line: String,
        path: usize,
        station: String,
    },
    #[error("corner '{corner}' does not change direction")]
    CollinearCorner { corner: String },
    #[error("corner '{corner}' reverses direction")]
    ReversingCorner { corner: String },
    #[error("corner '{corner}' radius does not fit its adjacent legs")]
    CornerRadiusTooLarge { corner: String },
    #[error("corner radii overlap on path {path} of line '{line}'")]
    CornerRadiiOverlap { line: String, path: usize },
    #[error("schematic paths contain overlapping legs")]
    OverlappingLegs,
    #[error("schematic geometry is outside the renderer's numeric range")]
    CoordinateRange,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Point {
    pub x: f64,
    pub y: f64,
}

impl From<SchematicPoint> for Point {
    fn from(value: SchematicPoint) -> Self {
        Self {
            x: value.x(),
            y: value.y(),
        }
    }
}

#[derive(Debug)]
pub(super) struct PreparedSchematic<'a> {
    pub line_width: f64,
    pub symbols: Vec<PreparedSymbol<'a>>,
    pub strokes: Vec<PreparedStroke<'a>>,
}

#[derive(Debug)]
pub(super) struct PreparedSymbol<'a> {
    pub station: &'a SchematicStation,
    pub center: Point,
    pub shape: PreparedShape,
    pub fill: &'a str,
    pub stroke: &'a str,
    pub stroke_width: f64,
    pub stroke_alignment: crate::SchematicStrokeAlignment,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PreparedShape {
    Circle {
        diameter: f64,
    },
    Capsule {
        axis: OctilinearAxis,
        diameter: f64,
        length: f64,
    },
}

#[derive(Debug)]
pub(super) struct PreparedStroke<'a> {
    pub id: &'a str,
    pub color: &'a str,
    pub paths: Vec<PreparedPath<'a>>,
}

#[derive(Debug)]
pub(super) struct PreparedPath<'a> {
    pub points: Vec<PreparedPathPoint<'a>>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PreparedPathPoint<'a> {
    pub position: Point,
    pub kind: PreparedPointKind<'a>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PreparedPointKind<'a> {
    Anchor {
        station: &'a str,
        permitted_axis: Option<OctilinearAxis>,
    },
    Corner {
        id: &'a str,
        radius: f64,
    },
}

/// Validate all semantic and geometric invariants required by the schematic renderer.
pub fn validate_schematic(schematic: &SchematicManifest) -> Result<(), SchematicRenderError> {
    let prepared = prepare_schematic(schematic)?;
    crate::schematic_render::validate_render_bounds(&prepared)
}

pub(super) fn prepare_schematic(
    schematic: &SchematicManifest,
) -> Result<PreparedSchematic<'_>, SchematicRenderError> {
    let stations = station_index(schematic)?;
    let corners = corner_index(schematic)?;
    let mut line_ids = HashSet::with_capacity(schematic.lines.len());
    let mut corner_owners = HashMap::<&str, &str>::new();
    let mut port_owners = HashMap::<PortKey<'_>, &str>::new();
    let mut port_references = HashMap::<PortKey<'_>, usize>::new();
    let mut station_lines = HashMap::<&str, &str>::new();
    let mut strokes = Vec::with_capacity(schematic.lines.len());

    for line in &schematic.lines {
        if line.id.trim().is_empty() {
            return Err(SchematicRenderError::EmptyLineId);
        }
        if !line_ids.insert(line.id.as_str()) {
            return Err(SchematicRenderError::DuplicateLine {
                line: line.id.clone(),
            });
        }
        validate_names("line", &line.id, &line.names)?;

        let mut paths = Vec::with_capacity(line.paths.len());
        for (path_index, path) in line.paths.iter().enumerate() {
            let path_number = path_index + 1;
            let station_count = path
                .visits
                .iter()
                .filter(|visit| matches!(visit, SchematicRouteVisit::Station { .. }))
                .count();
            let minimum = if path.closed { 3 } else { 2 };
            if station_count < minimum {
                return Err(SchematicRenderError::PathTooShort {
                    line: line.id.clone(),
                    path: path_number,
                    minimum,
                });
            }
            if !path.closed
                && (!matches!(
                    path.visits.first(),
                    Some(SchematicRouteVisit::Station { .. })
                ) || !matches!(
                    path.visits.last(),
                    Some(SchematicRouteVisit::Station { .. })
                ))
            {
                return Err(SchematicRenderError::OpenPathEndpoint {
                    line: line.id.clone(),
                    path: path_number,
                });
            }

            let mut path_stations = HashSet::new();
            let mut path_corners = HashSet::new();
            let mut points = Vec::with_capacity(path.visits.len());
            for visit in &path.visits {
                match visit {
                    SchematicRouteVisit::Station { station_id, port } => {
                        if !path_stations.insert(station_id.as_str()) {
                            return Err(SchematicRenderError::DuplicateStationInPath {
                                line: line.id.clone(),
                                path: path_number,
                                station: station_id.clone(),
                            });
                        }
                        let station = stations.get(station_id.as_str()).ok_or_else(|| {
                            SchematicRenderError::UnknownStation {
                                line: line.id.clone(),
                                station: station_id.clone(),
                            }
                        })?;
                        let key = PortKey::new(station_id, *port);
                        if let Some(owner) = port_owners.insert(key, &line.id)
                            && owner != line.id
                        {
                            return Err(SchematicRenderError::PortSharedByLines {
                                port: key.display(),
                                first_line: owner.to_owned(),
                                second_line: line.id.clone(),
                            });
                        }
                        *port_references.entry(key).or_default() += 1;
                        station_lines.entry(station_id).or_insert(&line.id);
                        points.push(resolve_port(station, *port)?);
                    }
                    SchematicRouteVisit::Corner { corner_id } => {
                        if !path_corners.insert(corner_id.as_str()) {
                            return Err(SchematicRenderError::DuplicateCornerInPath {
                                line: line.id.clone(),
                                path: path_number,
                                corner: corner_id.clone(),
                            });
                        }
                        let corner = corners.get(corner_id.as_str()).ok_or_else(|| {
                            SchematicRenderError::UnknownCorner {
                                line: line.id.clone(),
                                corner: corner_id.clone(),
                            }
                        })?;
                        if let Some(owner) = corner_owners.insert(corner_id, &line.id)
                            && owner != line.id
                        {
                            return Err(SchematicRenderError::CornerSharedByLines {
                                corner: corner_id.clone(),
                                first_line: owner.to_owned(),
                                second_line: line.id.clone(),
                            });
                        }
                        points.push(PreparedPathPoint {
                            position: corner.position.into(),
                            kind: PreparedPointKind::Corner {
                                id: &corner.id,
                                radius: corner.radius.get(),
                            },
                        });
                    }
                }
            }
            validate_path(&line.id, path_number, &points, path.closed)?;
            paths.push(PreparedPath {
                points,
                closed: path.closed,
            });
        }
        strokes.push(PreparedStroke {
            id: &line.id,
            color: &line.color,
            paths,
        });
    }

    for corner in &schematic.corners {
        if !corner_owners.contains_key(corner.id.as_str()) {
            return Err(SchematicRenderError::UnreferencedCorner {
                corner: corner.id.clone(),
            });
        }
    }
    validate_perpendicular_references(schematic, &port_references)?;

    let line_width = schematic.options.lines.width.get();
    let mut symbols = Vec::with_capacity(schematic.stations.len());
    for station in &schematic.stations {
        let center = station.position.into();
        match station.symbol {
            SchematicStationSymbol::Circle {} => {
                let options = &schematic.options.stations.common;
                let fill = match &options.fill.color {
                    SchematicStationColor::Unified { value } => value.as_str(),
                    SchematicStationColor::FollowLine {} => station_lines
                        .get(station.id.as_str())
                        .and_then(|line_id| {
                            schematic.lines.iter().find(|line| line.id == **line_id)
                        })
                        .map_or("currentColor", |line| line.color.as_str()),
                };
                let stroke = match &options.stroke.color {
                    SchematicStationColor::Unified { value } => value.as_str(),
                    SchematicStationColor::FollowLine {} => station_lines
                        .get(station.id.as_str())
                        .and_then(|line_id| {
                            schematic.lines.iter().find(|line| line.id == **line_id)
                        })
                        .map_or("currentColor", |line| line.color.as_str()),
                };
                symbols.push(PreparedSymbol {
                    station,
                    center,
                    shape: PreparedShape::Circle {
                        diameter: options.fill.diameter.get().max(line_width),
                    },
                    fill,
                    stroke,
                    stroke_width: options.stroke.width.get(),
                    stroke_alignment: options.stroke.alignment,
                });
            }
            SchematicStationSymbol::Capsule {
                axis,
                anchor_count,
                anchor_interval,
            } => {
                if anchor_count > 1 && anchor_interval.get() < line_width {
                    return Err(SchematicRenderError::AnchorIntervalTooSmall {
                        station: station.id.clone(),
                    });
                }
                let options = &schematic.options.stations.interchange;
                let diameter = options.fill.width.get().max(line_width);
                let length =
                    diameter + f64::from(anchor_count.saturating_sub(1)) * anchor_interval.get();
                if !length.is_finite() {
                    return Err(SchematicRenderError::CoordinateRange);
                }
                symbols.push(PreparedSymbol {
                    station,
                    center,
                    shape: PreparedShape::Capsule {
                        axis,
                        diameter,
                        length,
                    },
                    fill: &options.fill.color,
                    stroke: &options.stroke.color,
                    stroke_width: options.stroke.width.get(),
                    stroke_alignment: options.stroke.alignment,
                });
            }
        }
    }

    validate_non_overlapping_legs(&strokes)?;

    Ok(PreparedSchematic {
        line_width,
        symbols,
        strokes,
    })
}

fn station_index(
    schematic: &SchematicManifest,
) -> Result<HashMap<&str, &SchematicStation>, SchematicRenderError> {
    let mut stations = HashMap::with_capacity(schematic.stations.len());
    for station in &schematic.stations {
        if station.id.trim().is_empty() {
            return Err(SchematicRenderError::EmptyStationId);
        }
        if stations.insert(station.id.as_str(), station).is_some() {
            return Err(SchematicRenderError::DuplicateStation {
                station: station.id.clone(),
            });
        }
        validate_names("station", &station.id, &station.names)?;
    }
    Ok(stations)
}

fn corner_index(
    schematic: &SchematicManifest,
) -> Result<HashMap<&str, &crate::SchematicCorner>, SchematicRenderError> {
    let mut corners = HashMap::with_capacity(schematic.corners.len());
    for corner in &schematic.corners {
        if corner.id.trim().is_empty() {
            return Err(SchematicRenderError::EmptyCornerId);
        }
        if corners.insert(corner.id.as_str(), corner).is_some() {
            return Err(SchematicRenderError::DuplicateCorner {
                corner: corner.id.clone(),
            });
        }
    }
    Ok(corners)
}

fn validate_names(
    kind: &'static str,
    id: &str,
    names: &crate::LocalizedNames,
) -> Result<(), SchematicRenderError> {
    for (locale, values) in names {
        if locale.trim().is_empty() {
            return Err(SchematicRenderError::EmptyLocale {
                kind,
                id: id.to_owned(),
            });
        }
        if values.first().is_none_or(|value| value.trim().is_empty()) {
            return Err(SchematicRenderError::EmptyCanonicalName {
                kind,
                id: id.to_owned(),
                locale: locale.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PortKey<'a> {
    station: &'a str,
    port: SchematicStationPort,
}

impl<'a> PortKey<'a> {
    fn new(station: &'a str, port: SchematicStationPort) -> Self {
        Self { station, port }
    }

    fn display(self) -> String {
        format!("{}:{:?}", self.station, self.port)
    }
}

fn resolve_port<'a>(
    station: &'a SchematicStation,
    port: SchematicStationPort,
) -> Result<PreparedPathPoint<'a>, SchematicRenderError> {
    let (position, permitted_axis) = match (station.symbol, port) {
        (SchematicStationSymbol::Circle {}, SchematicStationPort::SingleLine) => {
            (station.position.into(), None)
        }
        (
            SchematicStationSymbol::Capsule {
                axis,
                anchor_count,
                anchor_interval,
            },
            SchematicStationPort::Interchange(interchange),
        ) => {
            let (offset, axis) = match interchange {
                SchematicInterchangePort::MajorAxis {} => (0.0, axis),
                SchematicInterchangePort::RisingOblique {} => {
                    if anchor_count > 1 {
                        return Err(SchematicRenderError::ObliquePortWithMultipleAnchors {
                            station: station.id.clone(),
                            anchor_count,
                        });
                    }
                    (0.0, rotate_counter_clockwise(axis))
                }
                SchematicInterchangePort::FallingOblique {} => {
                    if anchor_count > 1 {
                        return Err(SchematicRenderError::ObliquePortWithMultipleAnchors {
                            station: station.id.clone(),
                            anchor_count,
                        });
                    }
                    (0.0, rotate_clockwise(axis))
                }
                SchematicInterchangePort::SinglePerpendicular {} => {
                    if anchor_count != 1 {
                        return Err(SchematicRenderError::IncompatiblePerpendicularPort {
                            station: station.id.clone(),
                            anchor_count,
                        });
                    }
                    (0.0, perpendicular(axis))
                }
                SchematicInterchangePort::PerpendicularAnchor { index } => {
                    if anchor_count <= 1 {
                        return Err(SchematicRenderError::IncompatiblePerpendicularPort {
                            station: station.id.clone(),
                            anchor_count,
                        });
                    }
                    if index >= anchor_count {
                        return Err(SchematicRenderError::PerpendicularAnchorOutOfRange {
                            station: station.id.clone(),
                            index,
                            anchor_count,
                        });
                    }
                    (
                        (f64::from(index) - (f64::from(anchor_count) - 1.0) / 2.0)
                            * anchor_interval.get(),
                        perpendicular(axis),
                    )
                }
            };
            let (unit_x, unit_y) = axis_vector(axis_of_capsule(station));
            let center = Point::from(station.position);
            (
                Point {
                    x: center.x + unit_x * offset,
                    y: center.y + unit_y * offset,
                },
                Some(axis),
            )
        }
        _ => {
            return Err(SchematicRenderError::IncompatiblePort {
                station: station.id.clone(),
            });
        }
    };
    if !position.x.is_finite() || !position.y.is_finite() {
        return Err(SchematicRenderError::CoordinateRange);
    }
    Ok(PreparedPathPoint {
        position,
        kind: PreparedPointKind::Anchor {
            station: &station.id,
            permitted_axis,
        },
    })
}

fn axis_of_capsule(station: &SchematicStation) -> OctilinearAxis {
    match station.symbol {
        SchematicStationSymbol::Capsule { axis, .. } => axis,
        SchematicStationSymbol::Circle {} => unreachable!("called only for a capsule"),
    }
}

fn validate_perpendicular_references(
    schematic: &SchematicManifest,
    references: &HashMap<PortKey<'_>, usize>,
) -> Result<(), SchematicRenderError> {
    for station in &schematic.stations {
        let SchematicStationSymbol::Capsule { anchor_count, .. } = station.symbol else {
            continue;
        };
        let valid = match anchor_count {
            0 => true,
            1 => {
                references.get(&PortKey::new(
                    &station.id,
                    SchematicStationPort::Interchange(
                        SchematicInterchangePort::SinglePerpendicular {},
                    ),
                )) == Some(&1)
            }
            count => (0..count).all(|index| {
                references.get(&PortKey::new(
                    &station.id,
                    SchematicStationPort::Interchange(
                        SchematicInterchangePort::PerpendicularAnchor { index },
                    ),
                )) == Some(&1)
            }),
        };
        if !valid {
            return Err(SchematicRenderError::IncompletePerpendicularPorts {
                station: station.id.clone(),
                anchor_count,
            });
        }
    }
    Ok(())
}

fn validate_path(
    line: &str,
    path: usize,
    points: &[PreparedPathPoint<'_>],
    closed: bool,
) -> Result<(), SchematicRenderError> {
    let segment_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    let mut axes = Vec::with_capacity(segment_count);
    for index in 0..segment_count {
        let next = (index + 1) % points.len();
        let dx = points[next].position.x - points[index].position.x;
        let dy = points[next].position.y - points[index].position.y;
        if nearly_zero(dx) && nearly_zero(dy) {
            return Err(SchematicRenderError::CoincidentPoints {
                line: line.to_owned(),
                path,
            });
        }
        axes.push(axis_for_delta(dx, dy).ok_or_else(|| {
            SchematicRenderError::NonOctilinearLeg {
                line: line.to_owned(),
                path,
            }
        })?);
    }

    for (index, point) in points.iter().enumerate() {
        let incoming = if index > 0 {
            Some(axes[index - 1])
        } else if closed {
            axes.last().copied()
        } else {
            None
        };
        let outgoing = if index < axes.len() {
            Some(axes[index])
        } else {
            None
        };
        match point.kind {
            PreparedPointKind::Anchor {
                station,
                permitted_axis,
            } => {
                if let Some(required) = permitted_axis
                    && incoming
                        .into_iter()
                        .chain(outgoing)
                        .any(|axis| axis != required)
                {
                    return Err(SchematicRenderError::InvalidPortAxis {
                        station: station.to_owned(),
                    });
                }
                if let (Some(before), Some(after)) = (incoming, outgoing)
                    && (before != after || anchor_reverses(points, index, closed))
                {
                    return Err(SchematicRenderError::BendAtStation {
                        line: line.to_owned(),
                        path,
                        station: station.to_owned(),
                    });
                }
            }
            PreparedPointKind::Corner { id, radius } => {
                let (Some(_), Some(_)) = (incoming, outgoing) else {
                    return Err(SchematicRenderError::CollinearCorner {
                        corner: id.to_owned(),
                    });
                };
                validate_corner(points, index, closed, id, radius)?;
            }
        }
    }
    for index in 0..segment_count {
        let next = (index + 1) % points.len();
        let length = distance(points[index].position, points[next].position);
        let first_cut = corner_cut(points, index, closed);
        let second_cut = corner_cut(points, next, closed);
        if first_cut + second_cut >= length - EPSILON {
            return Err(SchematicRenderError::CornerRadiiOverlap {
                line: line.to_owned(),
                path,
            });
        }
    }
    Ok(())
}

fn anchor_reverses(points: &[PreparedPathPoint<'_>], index: usize, closed: bool) -> bool {
    if !closed && (index == 0 || index + 1 == points.len()) {
        return false;
    }
    let previous = points[(index + points.len() - 1) % points.len()].position;
    let current = points[index].position;
    let next = points[(index + 1) % points.len()].position;
    let incoming_x = current.x - previous.x;
    let incoming_y = current.y - previous.y;
    let outgoing_x = next.x - current.x;
    let outgoing_y = next.y - current.y;
    incoming_x * outgoing_x + incoming_y * outgoing_y < 0.0
}

fn corner_cut(points: &[PreparedPathPoint<'_>], index: usize, closed: bool) -> f64 {
    let PreparedPointKind::Corner { radius, .. } = points[index].kind else {
        return 0.0;
    };
    if !closed && (index == 0 || index + 1 == points.len()) {
        return 0.0;
    }
    let previous = points[(index + points.len() - 1) % points.len()].position;
    let corner = points[index].position;
    let next = points[(index + 1) % points.len()].position;
    let incoming = unit(corner.x - previous.x, corner.y - previous.y);
    let outgoing = unit(next.x - corner.x, next.y - corner.y);
    let internal_angle = (-incoming.0 * outgoing.0 - incoming.1 * outgoing.1)
        .clamp(-1.0, 1.0)
        .acos();
    radius / (internal_angle / 2.0).tan()
}

fn validate_non_overlapping_legs(
    strokes: &[PreparedStroke<'_>],
) -> Result<(), SchematicRenderError> {
    let mut legs = Vec::new();
    for stroke in strokes {
        for path in &stroke.paths {
            let count = if path.closed {
                path.points.len()
            } else {
                path.points.len().saturating_sub(1)
            };
            for index in 0..count {
                legs.push((
                    path.points[index].position,
                    path.points[(index + 1) % path.points.len()].position,
                ));
            }
        }
    }
    for (index, &(first_start, first_end)) in legs.iter().enumerate() {
        for &(second_start, second_end) in &legs[index + 1..] {
            if legs_overlap(first_start, first_end, second_start, second_end) {
                return Err(SchematicRenderError::OverlappingLegs);
            }
        }
    }
    Ok(())
}

fn legs_overlap(
    first_start: Point,
    first_end: Point,
    second_start: Point,
    second_end: Point,
) -> bool {
    let dx = first_end.x - first_start.x;
    let dy = first_end.y - first_start.y;
    let offset_x = second_start.x - first_start.x;
    let offset_y = second_start.y - first_start.y;
    let scale = dx.abs().max(dy.abs()).max(1.0);
    if (dx * offset_y - dy * offset_x).abs() > EPSILON * scale * scale {
        return false;
    }
    let second_dx = second_end.x - second_start.x;
    let second_dy = second_end.y - second_start.y;
    if (dx * second_dy - dy * second_dx).abs() > EPSILON * scale * scale {
        return false;
    }
    let (first_min, first_max, second_min, second_max) = if dx.abs() >= dy.abs() {
        (
            first_start.x.min(first_end.x),
            first_start.x.max(first_end.x),
            second_start.x.min(second_end.x),
            second_start.x.max(second_end.x),
        )
    } else {
        (
            first_start.y.min(first_end.y),
            first_start.y.max(first_end.y),
            second_start.y.min(second_end.y),
            second_start.y.max(second_end.y),
        )
    };
    first_max.min(second_max) - first_min.max(second_min) > EPSILON * scale
}

fn validate_corner(
    points: &[PreparedPathPoint<'_>],
    index: usize,
    closed: bool,
    id: &str,
    radius: f64,
) -> Result<(), SchematicRenderError> {
    if !closed && (index == 0 || index + 1 == points.len()) {
        return Err(SchematicRenderError::CollinearCorner {
            corner: id.to_owned(),
        });
    }
    let previous = points[(index + points.len() - 1) % points.len()].position;
    let current = points[index].position;
    let next = points[(index + 1) % points.len()].position;
    let incoming = unit(current.x - previous.x, current.y - previous.y);
    let outgoing = unit(next.x - current.x, next.y - current.y);
    let cross = incoming.0 * outgoing.1 - incoming.1 * outgoing.0;
    let dot = incoming.0 * outgoing.0 + incoming.1 * outgoing.1;
    if nearly_zero(cross) {
        return Err(if dot > 0.0 {
            SchematicRenderError::CollinearCorner {
                corner: id.to_owned(),
            }
        } else {
            SchematicRenderError::ReversingCorner {
                corner: id.to_owned(),
            }
        });
    }
    let internal_angle = (-incoming.0 * outgoing.0 - incoming.1 * outgoing.1)
        .clamp(-1.0, 1.0)
        .acos();
    let tangent = radius / (internal_angle / 2.0).tan();
    let previous_length = distance(previous, current);
    let next_length = distance(current, next);
    if !tangent.is_finite()
        || tangent >= previous_length - EPSILON
        || tangent >= next_length - EPSILON
    {
        return Err(SchematicRenderError::CornerRadiusTooLarge {
            corner: id.to_owned(),
        });
    }
    Ok(())
}

pub(super) fn corner_tangents(
    previous: Point,
    corner: Point,
    next: Point,
    radius: f64,
) -> (Point, Point, bool) {
    let incoming = unit(corner.x - previous.x, corner.y - previous.y);
    let outgoing = unit(next.x - corner.x, next.y - corner.y);
    let internal_angle = (-incoming.0 * outgoing.0 - incoming.1 * outgoing.1)
        .clamp(-1.0, 1.0)
        .acos();
    let tangent = radius / (internal_angle / 2.0).tan();
    (
        Point {
            x: corner.x - incoming.0 * tangent,
            y: corner.y - incoming.1 * tangent,
        },
        Point {
            x: corner.x + outgoing.0 * tangent,
            y: corner.y + outgoing.1 * tangent,
        },
        incoming.0 * outgoing.1 - incoming.1 * outgoing.0 > 0.0,
    )
}

fn axis_for_delta(dx: f64, dy: f64) -> Option<OctilinearAxis> {
    let scale = dx.abs().max(dy.abs()).max(1.0);
    if dx.abs() <= EPSILON * scale {
        Some(OctilinearAxis::Vertical)
    } else if dy.abs() <= EPSILON * scale {
        Some(OctilinearAxis::Horizontal)
    } else if (dx.abs() - dy.abs()).abs() <= EPSILON * scale {
        if dx * dy > 0.0 {
            Some(OctilinearAxis::FallingDiagonal)
        } else {
            Some(OctilinearAxis::RisingDiagonal)
        }
    } else {
        None
    }
}

fn rotate_counter_clockwise(axis: OctilinearAxis) -> OctilinearAxis {
    match axis {
        OctilinearAxis::Horizontal => OctilinearAxis::RisingDiagonal,
        OctilinearAxis::FallingDiagonal => OctilinearAxis::Horizontal,
        OctilinearAxis::Vertical => OctilinearAxis::FallingDiagonal,
        OctilinearAxis::RisingDiagonal => OctilinearAxis::Vertical,
    }
}

fn rotate_clockwise(axis: OctilinearAxis) -> OctilinearAxis {
    match axis {
        OctilinearAxis::Horizontal => OctilinearAxis::FallingDiagonal,
        OctilinearAxis::FallingDiagonal => OctilinearAxis::Vertical,
        OctilinearAxis::Vertical => OctilinearAxis::RisingDiagonal,
        OctilinearAxis::RisingDiagonal => OctilinearAxis::Horizontal,
    }
}

fn perpendicular(axis: OctilinearAxis) -> OctilinearAxis {
    match axis {
        OctilinearAxis::Horizontal => OctilinearAxis::Vertical,
        OctilinearAxis::FallingDiagonal => OctilinearAxis::RisingDiagonal,
        OctilinearAxis::Vertical => OctilinearAxis::Horizontal,
        OctilinearAxis::RisingDiagonal => OctilinearAxis::FallingDiagonal,
    }
}

pub(super) fn axis_vector(axis: OctilinearAxis) -> (f64, f64) {
    const DIAGONAL: f64 = std::f64::consts::FRAC_1_SQRT_2;
    match axis {
        OctilinearAxis::Horizontal => (1.0, 0.0),
        OctilinearAxis::FallingDiagonal => (DIAGONAL, DIAGONAL),
        OctilinearAxis::Vertical => (0.0, 1.0),
        OctilinearAxis::RisingDiagonal => (DIAGONAL, -DIAGONAL),
    }
}

fn unit(dx: f64, dy: f64) -> (f64, f64) {
    let length = dx.hypot(dy);
    (dx / length, dy / length)
}

fn distance(first: Point, second: Point) -> f64 {
    (second.x - first.x).hypot(second.y - first.y)
}

fn nearly_zero(value: f64) -> bool {
    value.abs() <= EPSILON
}
