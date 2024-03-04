pub type Waypoint = crate::plugins::PlayerCoordinates;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct WaypointSave {
    pub most_recent: Option<Waypoint>,
}
