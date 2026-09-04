//! Core data models for diagrammatic city metro topology and schematic maps.
//!
//! Topology coordinates are deliberately abstract Cartesian coordinates.
//! Schematic coordinates follow SVG's rightward x-axis and downward y-axis.
//!
//! YAML is the primary, human-editable manifest format. JSON uses the same
//! schemas and is available for interchange with web applications.

mod manifest_format;
mod schematic;
mod schematic_render;
mod schematic_validation;
mod topology;
mod topology_layout;
mod topology_render;
mod topology_validation;

use std::collections::BTreeMap;

pub use schematic::{
    OctilinearAxis, SchematicBackgroundOptions, SchematicCommonStationFill,
    SchematicCommonStationOptions, SchematicCommonStationStroke, SchematicCorner,
    SchematicInterchangePort, SchematicInterchangeStationFill, SchematicInterchangeStationOptions,
    SchematicInterchangeStationStroke, SchematicLength, SchematicLine, SchematicLineOptions,
    SchematicManifest, SchematicOptions, SchematicPath, SchematicPoint, SchematicRouteVisit,
    SchematicStation, SchematicStationColor, SchematicStationOptions, SchematicStationPort,
    SchematicStationSymbol, SchematicStrokeAlignment, SchematicValueError,
};
pub use schematic_render::render_schematic_svg;
pub use schematic_validation::{SchematicRenderError, validate_schematic};
pub use topology::{MetroTopology, TopologyLine, TopologyPath, TopologyPosition, TopologyStation};
pub use topology_render::render_topology_svg;
pub use topology_validation::{TopologyRenderError, validate_topology};

/// Names indexed by a locale such as `en` or `zh-CN`.
///
/// The first entry for a locale is its canonical name. Every following entry
/// is an alias in that locale.
pub type LocalizedNames = BTreeMap<String, Vec<String>>;
