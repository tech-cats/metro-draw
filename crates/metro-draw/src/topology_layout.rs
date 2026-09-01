use crate::{MetroTopology, TopologyStation};

const SCALE: f64 = 80.0;
const PADDING: f64 = 48.0;
const LABEL_SPACE: f64 = 160.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

impl Bounds {
    pub(super) fn from_topology(topology: &MetroTopology) -> Self {
        let Some(first) = topology.stations.first() else {
            return Self {
                min_x: 0.0,
                max_x: 0.0,
                min_y: 0.0,
                max_y: 0.0,
            };
        };

        let mut bounds = Self {
            min_x: first.position.x,
            max_x: first.position.x,
            min_y: first.position.y,
            max_y: first.position.y,
        };
        for station in &topology.stations[1..] {
            bounds.min_x = bounds.min_x.min(station.position.x);
            bounds.max_x = bounds.max_x.max(station.position.x);
            bounds.min_y = bounds.min_y.min(station.position.y);
            bounds.max_y = bounds.max_y.max(station.position.y);
        }
        bounds
    }

    pub(super) fn viewport(self) -> Option<(f64, f64)> {
        let width = (self.max_x - self.min_x) * SCALE + PADDING * 2.0 + LABEL_SPACE;
        let height = (self.max_y - self.min_y) * SCALE + PADDING * 2.0;

        (width.is_finite() && height.is_finite()).then_some((width, height))
    }

    pub(super) fn project(self, station: &TopologyStation) -> Option<(f64, f64)> {
        let x = (station.position.x - self.min_x) * SCALE + PADDING;
        let y = (self.max_y - station.position.y) * SCALE + PADDING;

        (x.is_finite() && y.is_finite()).then_some((x, y))
    }
}
