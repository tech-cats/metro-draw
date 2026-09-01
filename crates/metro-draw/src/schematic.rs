use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{LocalizedNames, manifest_format::inline_yaml_positions};

/// A complete, human-editable schematic-map manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicManifest {
    pub options: SchematicOptions,
    pub stations: Vec<SchematicStation>,
    pub corners: Vec<SchematicCorner>,
    pub lines: Vec<SchematicLine>,
}

/// Global dimensions used by a schematic map.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicOptions {
    pub line_width: SchematicLength,
    pub station_diameter: SchematicLength,
}

/// A station and its requested schematic symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicStation {
    pub id: String,
    pub position: SchematicPoint,
    pub names: LocalizedNames,
    pub symbol: SchematicStationSymbol,
}

/// The station symbol requested by the semantic schematic manifest.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchematicStationSymbol {
    // Empty struct variants make `deny_unknown_fields` apply to fieldless cases.
    Circle {},
    Capsule {
        axis: OctilinearAxis,
        anchor_count: u8,
        anchor_interval: SchematicLength,
    },
}

/// A layout corner shared by its semantic reference and geometric result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicCorner {
    pub id: String,
    pub position: SchematicPoint,
    pub radius: SchematicLength,
}

/// A semantic metro line and its schematic paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicLine {
    pub id: String,
    pub names: LocalizedNames,
    pub color: String,
    pub paths: Vec<SchematicPath>,
}

/// An ordered semantic traversal of stations and layout corners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicPath {
    pub visits: Vec<SchematicRouteVisit>,
    pub closed: bool,
}

/// A station or corner visited by a semantic schematic path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchematicRouteVisit {
    Station {
        station_id: String,
        port: SchematicStationPort,
    },
    Corner {
        corner_id: String,
    },
}

/// A route's connection to a common or interchange station.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "interchange",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchematicStationPort {
    SingleLine,
    Interchange(SchematicInterchangePort),
}

/// A route's geometric relationship to an interchange capsule's major axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchematicInterchangePort {
    // Empty struct variants make `deny_unknown_fields` apply to fieldless cases.
    MajorAxis {},
    RisingOblique {},
    FallingOblique {},
    SinglePerpendicular {},
    PerpendicularAnchor { index: u8 },
}

/// An undirected octilinear axis in SVG coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OctilinearAxis {
    Horizontal,
    FallingDiagonal,
    Vertical,
    RisingDiagonal,
}

/// A finite point in the schematic coordinate system.
///
/// It is serialized and deserialized as `[x, y]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchematicPoint {
    x: f64,
    y: f64,
}

impl SchematicPoint {
    /// Construct a point whose coordinates are both finite.
    pub fn new(x: f64, y: f64) -> Result<Self, SchematicValueError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(SchematicValueError::NonFinitePoint { x, y });
        }

        Ok(Self { x, y })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }
}

impl Serialize for SchematicPoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        [self.x, self.y].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SchematicPoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [x, y] = <[f64; 2]>::deserialize(deserializer)?;
        Self::new(x, y).map_err(serde::de::Error::custom)
    }
}

/// A finite, strictly positive schematic length.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SchematicLength(f64);

impl SchematicLength {
    /// Construct a finite, strictly positive length.
    pub fn new(value: f64) -> Result<Self, SchematicValueError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(SchematicValueError::InvalidLength(value));
        }

        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for SchematicLength {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SchematicLength {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// A scalar invariant violation while constructing schematic values.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum SchematicValueError {
    #[error("schematic point coordinates must be finite, got [{x}, {y}]")]
    NonFinitePoint { x: f64, y: f64 },
    #[error("schematic length must be finite and strictly positive, got {0}")]
    InvalidLength(f64),
}

impl SchematicManifest {
    /// Deserialize a semantic schematic manifest from YAML.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Serialize a semantic schematic manifest to YAML.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self).map(inline_yaml_positions)
    }

    /// Deserialize a semantic schematic manifest from equivalent JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize a semantic schematic manifest as compact JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMATIC_YAML: &str = r##"
options:
  line_width: 8.0
  station_diameter: 18.0

stations:
  - id: west
    position: [0.0, 40.0]
    names:
      en:
        - West
    symbol:
      type: circle

  - id: central
    position: [80.0, 80.0]
    names:
      en:
        - Central
    symbol:
      type: capsule
      axis: rising_diagonal
      anchor_count: 1
      anchor_interval: 24.0

corners:
  - id: west-corner
    position: [40.0, 40.0]
    radius: 8.0

lines:
  - id: line-a
    names:
      en:
        - Line A
    color: "#e2231a"
    paths:
      - visits:
          - type: station
            station_id: west
            port:
              type: single_line
          - type: corner
            corner_id: west-corner
          - type: station
            station_id: central
            port:
              type: interchange
              interchange:
                type: single_perpendicular
        closed: false
"##;

    #[test]
    fn deserializes_schematic_station_ports() {
        let schematic = SchematicManifest::from_yaml(SCHEMATIC_YAML).unwrap();

        assert_eq!(schematic.options.line_width.get(), 8.0);
        assert_eq!(schematic.stations[0].position.x(), 0.0);
        assert_eq!(schematic.stations[0].position.y(), 40.0);
        assert_eq!(
            schematic.lines[0].paths[0].visits[2],
            SchematicRouteVisit::Station {
                station_id: "central".to_owned(),
                port: SchematicStationPort::Interchange(
                    SchematicInterchangePort::SinglePerpendicular {},
                ),
            }
        );
    }

    #[test]
    fn round_trips_schematic_manifest_through_yaml() {
        let schematic = SchematicManifest::from_yaml(SCHEMATIC_YAML).unwrap();
        let encoded = schematic.to_yaml().unwrap();
        let decoded = SchematicManifest::from_yaml(&encoded).unwrap();
        let value = serde_yaml::from_str::<serde_yaml::Value>(&encoded).unwrap();

        assert_eq!(decoded, schematic);
        assert!(encoded.contains("position: [0.0, 40.0]"));
        assert_eq!(
            value["lines"][0]["paths"][0]["visits"][2]["port"]["interchange"]["type"],
            "single_perpendicular"
        );
        assert!(value["lines"][0]["paths"][0]["visits"][2]["port"]["port"].is_null());
    }

    #[test]
    fn round_trips_schematic_manifest_through_json() {
        let schematic = SchematicManifest::from_yaml(SCHEMATIC_YAML).unwrap();
        let encoded = schematic.to_json().unwrap();
        let decoded = SchematicManifest::from_json(&encoded).unwrap();

        assert_eq!(decoded, schematic);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["lines"][0]["paths"][0]["visits"]
                [2]["port"]["interchange"]["type"],
            "single_perpendicular"
        );
    }

    #[test]
    fn rejects_repeated_port_key_for_interchange_payload() {
        let yaml = SCHEMATIC_YAML.replace(
            "              interchange:\n                type: single_perpendicular",
            "              port:\n                type: single_perpendicular",
        );

        assert!(SchematicManifest::from_yaml(&yaml).is_err());
    }

    #[test]
    fn rejects_invalid_schematic_scalars() {
        let non_finite_point =
            SCHEMATIC_YAML.replace("position: [0.0, 40.0]", "position: [.nan, 40.0]");
        let zero_length = SCHEMATIC_YAML.replace("line_width: 8.0", "line_width: 0.0");

        assert!(SchematicManifest::from_yaml(&non_finite_point).is_err());
        assert!(SchematicManifest::from_yaml(&zero_length).is_err());
        assert_eq!(
            SchematicLength::new(-1.0),
            Err(SchematicValueError::InvalidLength(-1.0))
        );
    }

    #[test]
    fn rejects_unknown_schematic_fields() {
        let invalid_port = SCHEMATIC_YAML.replace(
            "                type: single_perpendicular",
            "                type: single_perpendicular\n                index: 0",
        );
        let invalid_circle = SCHEMATIC_YAML.replace(
            "    symbol:\n      type: circle",
            "    symbol:\n      type: circle\n      diameter: 18.0",
        );
        let invalid_single_line = SCHEMATIC_YAML.replace(
            "              type: single_line",
            "              type: single_line\n              index: 0",
        );

        assert!(SchematicManifest::from_yaml(&invalid_port).is_err());
        assert!(SchematicManifest::from_yaml(&invalid_circle).is_err());
        assert!(SchematicManifest::from_yaml(&invalid_single_line).is_err());
    }
}
