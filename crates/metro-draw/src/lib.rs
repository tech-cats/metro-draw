//! Core data model for a diagrammatic city metro map.
//!
//! Coordinates are deliberately abstract Cartesian coordinates. They describe
//! where a station is drawn and are not longitude and latitude.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Names indexed by a locale such as `en` or `zh-CN`.
///
/// The first entry for a locale is its canonical name. Every following entry
/// is an alias in that locale.
pub type LocalizedNames = BTreeMap<String, Vec<String>>;

/// An entire metro map manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetroMap {
    pub stations: Vec<Station>,
    pub lines: Vec<Line>,
}

/// A station and its position on the drawing canvas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Station {
    pub id: String,
    pub names: LocalizedNames,
    pub position: Position,
}

/// A point in the map's abstract Cartesian coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// A metro line composed of one or more paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub id: String,
    pub names: LocalizedNames,
    pub paths: Vec<LinePath>,
}

/// An ordered traversal of stations belonging to a line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinePath {
    pub stations: Vec<String>,
    pub closed: bool,
}

impl MetroMap {
    /// Deserialize a metro map from a YAML manifest.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Serialize a metro map as a YAML manifest.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP_YAML: &str = r#"
stations:
  - id: futian
    names:
      zh-CN:
        - 福田
      en:
        - Futian
    position:
      x: 10.0
      y: 20.0

  - id: airport
    names:
      zh-CN:
        - 机场
      en:
        - Airport
    position:
      x: 90.0
      y: 20.0

lines:
  - id: line-11
    names:
      zh-CN:
        - 11 号线
        - 机场线
      en:
        - Line 11
        - Airport Express
    paths:
      - stations:
          - futian
          - airport
        closed: false
"#;

    #[test]
    fn deserializes_yaml_manifest() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();

        assert_eq!(map.stations.len(), 2);
        assert_eq!(map.stations[0].position, Position { x: 10.0, y: 20.0 });
        assert_eq!(map.lines[0].names["en"][0], "Line 11");
        assert_eq!(map.lines[0].names["en"][1], "Airport Express");
        assert!(!map.lines[0].paths[0].closed);
    }

    #[test]
    fn round_trips_through_yaml() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();
        let encoded = map.to_yaml().unwrap();
        let decoded = MetroMap::from_yaml(&encoded).unwrap();

        assert_eq!(decoded, map);
    }

    #[test]
    fn serde_model_also_round_trips_through_json() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();
        let encoded = serde_json::to_string(&map).unwrap();
        let decoded: MetroMap = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, map);
    }

    #[test]
    fn rejects_geographic_coordinate_fields() {
        let yaml = MAP_YAML
            .replace("x: 10.0", "longitude: 114.0")
            .replace("y: 20.0", "latitude: 22.0");
        let error = MetroMap::from_yaml(&yaml).unwrap_err();

        assert!(error.to_string().contains("unknown field `longitude`"));
    }
}
