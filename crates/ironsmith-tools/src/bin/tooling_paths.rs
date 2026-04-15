use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn repo_root() -> io::Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for candidate in manifest_dir.ancestors() {
        if looks_like_repo_root(candidate) {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(io::Error::other(format!(
        "failed to locate repo root from {}",
        manifest_dir.display()
    )))
}

fn looks_like_repo_root(candidate: &Path) -> bool {
    candidate.join("Cargo.toml").is_file()
        && candidate.join("build.rs").is_file()
        && candidate
            .join("scripts")
            .join("stream_scryfall_blocks.py")
            .is_file()
}
