//! Shared filesystem paths for the edge binary.

use std::path::{Path, PathBuf};

pub(crate) fn db_path(data_dir: &Path, edge_db_filename: &str) -> PathBuf {
    data_dir.join(edge_db_filename)
}
