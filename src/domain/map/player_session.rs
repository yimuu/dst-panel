//! Player session-file lookup helpers.

use std::{io, path::Path};

use crate::validation::{KuId, validate_ku_id};

use super::session::{
    LatestFileFilter, MapLevel, latest_direct_file_in_dir, latest_world_session_file,
    read_cluster_text, session_base_relative, session_id_from_world_relative_path,
};

/// Validated Klei user id used for player session-file lookup.
#[derive(Debug, Clone)]
pub struct MapKuId {
    ku_id: KuId,
}

impl MapKuId {
    /// Validates the `KU_...` value before building the legacy `${kuId}_` path.
    pub fn parse(ku_id: &str) -> io::Result<Self> {
        let ku_id = validate_ku_id(ku_id)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        Ok(Self { ku_id })
    }

    fn player_dir_name(&self) -> String {
        format!("{}_", self.ku_id.as_str())
    }
}

/// Reads the newest player session file under the current world session id.
pub fn read_latest_player_session_file(
    cluster_dir: &Path,
    level: &MapLevel,
    ku_id: &MapKuId,
) -> io::Result<String> {
    let world_relative_path = latest_world_session_file(cluster_dir, level)?;
    let session_id = session_id_from_world_relative_path(level, &world_relative_path)?;
    let player_dir = session_base_relative(level)
        .join(session_id)
        .join(ku_id.player_dir_name());
    let player_file = latest_direct_file_in_dir(cluster_dir, &player_dir, LatestFileFilter::Any)?;
    read_cluster_text(cluster_dir, &player_file)
}
