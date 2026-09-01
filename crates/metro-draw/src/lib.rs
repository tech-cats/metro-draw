//! Core data models for diagrammatic city metro topology and schematic maps.
//!
//! Topology coordinates are deliberately abstract Cartesian coordinates.
//! Schematic coordinates follow SVG's rightward x-axis and downward y-axis.
//!
//! YAML is the primary, human-editable manifest format. JSON uses the same
//! schemas and is available for interchange with web applications.

mod manifest_format;
mod schematic;
mod topology;
mod topology_layout;
mod topology_render;
mod topology_validation;

use std::collections::BTreeMap;

pub use schematic::{
    OctilinearAxis, SchematicCorner, SchematicInterchangePort, SchematicLength, SchematicLine,
    SchematicManifest, SchematicOptions, SchematicPath, SchematicPoint, SchematicRouteVisit,
    SchematicStation, SchematicStationPort, SchematicStationSymbol, SchematicValueError,
};
pub use topology::{MetroTopology, TopologyLine, TopologyPath, TopologyPosition, TopologyStation};
pub use topology_render::render_topology_svg;
pub use topology_validation::{TopologyRenderError, validate_topology};

/// Names indexed by a locale such as `en` or `zh-CN`.
///
/// The first entry for a locale is its canonical name. Every following entry
/// is an alias in that locale.
pub type LocalizedNames = BTreeMap<String, Vec<String>>;
