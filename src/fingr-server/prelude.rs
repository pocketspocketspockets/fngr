pub use anyhow::{Result, anyhow};
pub use tracing::{debug, error, info, subscriber, warn};

/// Helper function to return an error if path is relative using `Path::is_relative`. The check is disabled in a debug binary.
#[inline]
pub fn is_relative(name: &str, p: &std::path::Path) -> Result<()> {
    #[cfg(not(debug_assertions))]
    if p.is_relative() {
        return Err(anyhow!("{} path cannot be relative!", name));
    }

    #[cfg(debug_assertions)]
    {
        warn!(
            "debug_assertions: skipping relative path check for {}",
            name
        );
    }

    Ok(())
}
