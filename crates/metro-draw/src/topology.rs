use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{LocalizedNames, manifest_format::inline_yaml_positions};

/// An entire metro topology manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetroTopology {
    pub stations: Vec<TopologyStation>,
    pub lines: Vec<TopologyLine>,
}

/// A station and its position in the topology graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyStation {
    pub id: String,
    pub names: LocalizedNames,
    pub position: TopologyPosition,
}

/// A point in the topology's abstract Cartesian coordinate system.
///
/// It is serialized and deserialized as `[x, y]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopologyPosition {
    pub x: f64,
    pub y: f64,
}

impl Serialize for TopologyPosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        [self.x, self.y].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TopologyPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [x, y] = <[f64; 2]>::deserialize(deserializer)?;
        Ok(Self { x, y })
    }
}

/// A topological metro line composed of one or more paths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyLine {
    pub id: String,
    pub names: LocalizedNames,
    pub color: String,
    pub paths: Vec<TopologyPath>,
}

/// An ordered topological traversal of stations belonging to a line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyPath {
    pub stations: Vec<String>,
    pub closed: bool,
}

impl MetroTopology {
    /// Deserialize a metro topology from a YAML manifest.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Serialize a metro topology as a YAML manifest.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self).map(inline_yaml_positions)
    }

    /// Deserialize a metro topology from JSON using the same schema as YAML.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize a metro topology as compact JSON suitable for transport to a WebUI.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOPOLOGY_YAML: &str = r#"
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
        let topology = MetroTopology::from_yaml(TOPOLOGY_YAML).unwrap();

        assert_eq!(topology.stations.len(), 2);
        assert_eq!(
            topology.stations[0].position,
            TopologyPosition { x: 10.0, y: 20.0 }
        );
        assert_eq!(
            topology.stations[1].position,
            TopologyPosition { x: 90.0, y: 20.0 }
        );
        assert_eq!(topology.lines[0].names["en"][0], "Line 11");
        assert_eq!(topology.lines[0].names["en"][1], "Airport Express");
        assert_eq!(topology.lines[0].color, "#672146");
        assert!(!topology.lines[0].paths[0].closed);
    }

    #[test]
    fn round_trips_through_yaml() {
        let topology = MetroTopology::from_yaml(TOPOLOGY_YAML).unwrap();
        let encoded = topology.to_yaml().unwrap();
        let decoded = MetroTopology::from_yaml(&encoded).unwrap();

        assert_eq!(decoded, topology);
        assert!(encoded.contains("position: [10.0, 20.0]"));
        assert!(encoded.contains("position: [90.0, 20.0]"));
        assert!(!encoded.contains("position:\n"));
    }

    #[test]
    fn converts_from_yaml_to_json_and_back() {
        let topology = MetroTopology::from_yaml(TOPOLOGY_YAML).unwrap();
        let encoded = topology.to_json().unwrap();
        let decoded = MetroTopology::from_json(&encoded).unwrap();

        assert_eq!(decoded, topology);
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
        let topology = MetroTopology::from_yaml(TOPOLOGY_YAML).unwrap();
        let json = topology.to_json().unwrap();

        let yaml = MetroTopology::from_json(&json).unwrap().to_yaml().unwrap();

        assert_eq!(MetroTopology::from_yaml(&yaml).unwrap(), topology);
    }

    #[test]
    fn rejects_position_mapping() {
        let yaml = TOPOLOGY_YAML.replace(
            "position: [10.0, 20.0]",
            "position:\n      x: 10.0\n      y: 20.0",
        );

        assert!(MetroTopology::from_yaml(&yaml).is_err());
    }

    #[test]
    fn rejects_position_sequences_with_the_wrong_length() {
        let yaml = TOPOLOGY_YAML.replace("position: [90.0, 20.0]", "position: [90.0]");

        assert!(MetroTopology::from_yaml(&yaml).is_err());
    }
}
