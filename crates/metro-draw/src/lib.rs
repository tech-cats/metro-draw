//! Core data model for a diagrammatic city metro map.
//!
//! Coordinates are deliberately abstract Cartesian coordinates. They describe
//! where a station is drawn and are not longitude and latitude.
//!
//! YAML is the primary, human-editable manifest format. JSON uses the same
//! schema and is available for exchanging maps with web applications.

mod render;

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use render::{RenderError, render_topology_svg};

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
///
/// It is serialized and deserialized as `[x, y]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Serialize for Position {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        [self.x, self.y].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [x, y] = <[f64; 2]>::deserialize(deserializer)?;
        Ok(Self { x, y })
    }
}

/// A metro line composed of one or more paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub id: String,
    pub names: LocalizedNames,
    pub color: String,
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
        serde_yaml::to_string(self).map(inline_yaml_positions)
    }

    /// Deserialize a metro map from JSON using the same schema as YAML.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize a metro map as compact JSON suitable for transport to a WebUI.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// `serde_yaml` emits all sequences in block style. Position is the one place
/// where the manifest requires flow style to keep the document compact.
fn inline_yaml_positions(yaml: String) -> String {
    let lines: Vec<_> = yaml.lines().collect();
    let mut output = String::with_capacity(yaml.len());
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if line.trim() == "position:"
            && let (Some(x), Some(y)) = (lines.get(index + 1), lines.get(index + 2))
            && let (Some(x), Some(y)) = (x.trim().strip_prefix("- "), y.trim().strip_prefix("- "))
        {
            let indentation = &line[..line.len() - line.trim_start().len()];
            output.push_str(indentation);
            output.push_str("position: [");
            output.push_str(x);
            output.push_str(", ");
            output.push_str(y);
            output.push_str("]\n");
            index += 3;
            continue;
        }

        output.push_str(line);
        output.push('\n');
        index += 1;
    }

    output
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
    position: [10.0, 20.0]

  - id: airport
    names:
      zh-CN:
        - 机场
      en:
        - Airport
    position: [90.0, 20.0]

lines:
  - id: line-11
    names:
      zh-CN:
        - 11 号线
        - 机场线
      en:
        - Line 11
        - Airport Express
    color: '#672146'
    paths:
      - stations:
          - futian
          - airport
        closed: false
"#;

    #[test]
    fn deserializes_position_sequence() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();

        assert_eq!(map.stations.len(), 2);
        assert_eq!(map.stations[0].position, Position { x: 10.0, y: 20.0 });
        assert_eq!(map.stations[1].position, Position { x: 90.0, y: 20.0 });
        assert_eq!(map.lines[0].names["en"][0], "Line 11");
        assert_eq!(map.lines[0].names["en"][1], "Airport Express");
        assert_eq!(map.lines[0].color, "#672146");
        assert!(!map.lines[0].paths[0].closed);
    }

    #[test]
    fn round_trips_through_yaml() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();
        let encoded = map.to_yaml().unwrap();
        let decoded = MetroMap::from_yaml(&encoded).unwrap();

        assert_eq!(decoded, map);
        assert!(encoded.contains("position: [10.0, 20.0]"));
        assert!(encoded.contains("position: [90.0, 20.0]"));
        assert!(!encoded.contains("position:\n"));
    }

    #[test]
    fn converts_from_yaml_to_json_and_back() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();
        let encoded = map.to_json().unwrap();
        let decoded = MetroMap::from_json(&encoded).unwrap();

        assert_eq!(decoded, map);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["stations"][0]["position"],
            serde_json::json!([10.0, 20.0])
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["lines"][0]["names"]["en"]
                [0],
            "Line 11"
        );
    }

    #[test]
    fn converts_from_json_to_primary_yaml_format() {
        let map = MetroMap::from_yaml(MAP_YAML).unwrap();
        let json = map.to_json().unwrap();

        let yaml = MetroMap::from_json(&json).unwrap().to_yaml().unwrap();

        assert_eq!(MetroMap::from_yaml(&yaml).unwrap(), map);
    }

    #[test]
    fn rejects_position_mapping() {
        let yaml = MAP_YAML.replace(
            "position: [10.0, 20.0]",
            "position:\n      x: 10.0\n      y: 20.0",
        );

        assert!(MetroMap::from_yaml(&yaml).is_err());
    }

    #[test]
    fn rejects_position_sequences_with_the_wrong_length() {
        let yaml = MAP_YAML.replace("position: [90.0, 20.0]", "position: [90.0]");

        assert!(MetroMap::from_yaml(&yaml).is_err());
    }
}
