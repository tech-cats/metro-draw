use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeMap};

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

/// Global visual options used by a schematic map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicOptions {
    pub background: SchematicBackgroundOptions,
    pub lines: SchematicLineOptions,
    pub stations: SchematicStationOptions,
}

/// An opaque colour or a transparent map background.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchematicBackgroundOptions {
    Color { color: String },
    Transparent,
}

impl Serialize for SchematicBackgroundOptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::Color { color } => map.serialize_entry("color", color)?,
            Self::Transparent => map.serialize_entry("transparent", &true)?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for SchematicBackgroundOptions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            #[serde(alias = "colour")]
            color: Option<String>,
            transparent: Option<bool>,
        }

        match Fields::deserialize(deserializer)? {
            Fields {
                color: Some(color),
                transparent: None | Some(false),
            } => Ok(Self::Color { color }),
            Fields {
                color: None,
                transparent: Some(true),
            } => Ok(Self::Transparent),
            Fields {
                color: None,
                transparent: Some(false),
            } => Err(serde::de::Error::custom("transparent must be true")),
            Fields {
                color: Some(_),
                transparent: Some(true),
            } => Err(serde::de::Error::custom(
                "background color conflicts with transparent: true",
            )),
            Fields {
                color: None,
                transparent: None,
            } => Err(serde::de::Error::custom(
                "background must contain color or transparent: true",
            )),
        }
    }
}

/// Global styling shared by all metro lines.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicLineOptions {
    pub width: SchematicLength,
}

/// Global styling for common and interchange stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicStationOptions {
    pub common: SchematicCommonStationOptions,
    pub interchange: SchematicInterchangeStationOptions,
}

/// Styling shared by circle-shaped common stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicCommonStationOptions {
    pub fill: SchematicCommonStationFill,
    pub stroke: SchematicCommonStationStroke,
}

/// Fill styling for circle-shaped common stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicCommonStationFill {
    pub diameter: SchematicLength,
    #[serde(alias = "colour")]
    pub color: SchematicStationColor,
}

/// Stroke styling for circle-shaped common stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicCommonStationStroke {
    pub width: SchematicLength,
    pub alignment: SchematicStrokeAlignment,
    #[serde(alias = "colour")]
    pub color: SchematicStationColor,
}

/// Styling shared by capsule-shaped interchange stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicInterchangeStationOptions {
    pub fill: SchematicInterchangeStationFill,
    pub stroke: SchematicInterchangeStationStroke,
}

/// Fill styling for capsule-shaped interchange stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicInterchangeStationFill {
    pub width: SchematicLength,
    #[serde(alias = "colour")]
    pub color: String,
}

/// Stroke styling for capsule-shaped interchange stations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchematicInterchangeStationStroke {
    pub width: SchematicLength,
    pub alignment: SchematicStrokeAlignment,
    #[serde(alias = "colour")]
    pub color: String,
}

/// How a common-station colour is selected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum SchematicStationColor {
    Unified { value: String },
    // The empty struct makes `deny_unknown_fields` apply to this fieldless case.
    FollowLine {},
}

/// Placement of a station stroke relative to its fill boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchematicStrokeAlignment {
    Inside,
    #[serde(alias = "centre")]
    Center,
    Outside,
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
    #[serde(alias = "colour")]
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
  background:
    color: "#ffffff"
  lines:
    width: 8.0
  stations:
    common:
      fill:
        diameter: 18.0
        color:
          type: unified
          value: "#ffffff"
      stroke:
        width: 2.0
        alignment: centre
        color:
          type: follow-line
    interchange:
      fill:
        width: 18.0
        color: "#ffffff"
      stroke:
        width: 2.0
        alignment: outside
        color: "#000000"

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

        assert_eq!(schematic.options.lines.width.get(), 8.0);
        assert_eq!(
            schematic.options.background,
            SchematicBackgroundOptions::Color {
                color: "#ffffff".to_owned()
            }
        );
        assert_eq!(
            schematic.options.stations.common.stroke.alignment,
            SchematicStrokeAlignment::Center
        );
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
        assert!(encoded.contains("alignment: center"));
        assert!(!encoded.contains("alignment: centre"));
        assert_eq!(value["options"]["background"]["color"], "#ffffff");
        assert!(value["options"]["background"]["colour"].is_null());
        assert!(value["options"]["background"]["transparent"].is_null());
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
    fn accepts_british_spellings_and_serializes_canonically() {
        let british_yaml = SCHEMATIC_YAML.replace("color:", "colour:");
        let schematic = SchematicManifest::from_yaml(&british_yaml).unwrap();
        let canonical_yaml = schematic.to_yaml().unwrap();
        let british_json = schematic
            .to_json()
            .unwrap()
            .replace("\"color\":", "\"colour\":")
            .replace("\"center\"", "\"centre\"");

        assert_eq!(
            SchematicManifest::from_json(&british_json).unwrap(),
            schematic
        );
        assert!(canonical_yaml.contains("alignment: center"));
        assert!(canonical_yaml.contains("color:"));
        assert!(!canonical_yaml.contains("alignment: centre"));
        assert!(!canonical_yaml.contains("colour:"));
        assert!(
            schematic
                .to_json()
                .unwrap()
                .contains("\"alignment\":\"center\"")
        );
        assert!(schematic.to_json().unwrap().contains("\"color\":"));
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
        let zero_length = SCHEMATIC_YAML.replace("    width: 8.0", "    width: 0.0");

        assert!(SchematicManifest::from_yaml(&non_finite_point).is_err());
        assert!(SchematicManifest::from_yaml(&zero_length).is_err());
        assert_eq!(
            SchematicLength::new(-1.0),
            Err(SchematicValueError::InvalidLength(-1.0))
        );
    }

    #[test]
    fn enforces_exclusive_background_variants() {
        let color_background = "  background:\n    color: \"#ffffff\"";
        let transparent =
            SCHEMATIC_YAML.replace(color_background, "  background:\n    transparent: true");
        let decoded = SchematicManifest::from_yaml(&transparent).unwrap();
        let encoded = decoded.to_yaml().unwrap();
        let value = serde_yaml::from_str::<serde_yaml::Value>(&encoded).unwrap();

        assert_eq!(
            decoded.options.background,
            SchematicBackgroundOptions::Transparent
        );
        assert_eq!(value["options"]["background"]["transparent"], true);
        assert!(value["options"]["background"]["color"].is_null());
        for spelling in ["color", "colour"] {
            let opaque = SCHEMATIC_YAML.replace(
                color_background,
                &format!("  background:\n    {spelling}: \"#ffffff\"\n    transparent: false"),
            );
            let decoded = SchematicManifest::from_yaml(&opaque).unwrap();
            let canonical =
                serde_yaml::from_str::<serde_yaml::Value>(&decoded.to_yaml().unwrap()).unwrap();

            assert_eq!(
                decoded.options.background,
                SchematicBackgroundOptions::Color {
                    color: "#ffffff".to_owned()
                }
            );
            assert_eq!(canonical["options"]["background"]["color"], "#ffffff");
            assert!(canonical["options"]["background"]["transparent"].is_null());
            assert!(canonical["options"]["background"]["colour"].is_null());
        }
        assert!(
            SchematicManifest::from_yaml(&SCHEMATIC_YAML.replace(
                color_background,
                "  background:\n    color: \"#ffffff\"\n    transparent: true"
            ))
            .is_err()
        );
        assert!(
            SchematicManifest::from_yaml(
                &SCHEMATIC_YAML.replace(color_background, "  background:\n    transparent: false")
            )
            .is_err()
        );
        assert!(
            SchematicManifest::from_yaml(
                &SCHEMATIC_YAML.replace(color_background, "  background: {}")
            )
            .is_err()
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
        let invalid_color = SCHEMATIC_YAML.replace(
            "          type: follow-line",
            "          type: follow-line\n          value: \"#ffffff\"",
        );
        let obsolete_flat_option = SCHEMATIC_YAML.replace(
            "  lines:\n    width: 8.0",
            "  line_width: 8.0\n  lines:\n    width: 8.0",
        );

        assert!(SchematicManifest::from_yaml(&invalid_port).is_err());
        assert!(SchematicManifest::from_yaml(&invalid_circle).is_err());
        assert!(SchematicManifest::from_yaml(&invalid_single_line).is_err());
        assert!(SchematicManifest::from_yaml(&invalid_color).is_err());
        assert!(SchematicManifest::from_yaml(&obsolete_flat_option).is_err());
    }
}
