//! Reading what the phone's key service was asked to keep. The service
//! holds each key under a name, and what it holds is the handle the secure
//! world knows the key by.

use anyhow::{Context, Result};
use std::path::Path;

/// Where the key service keeps what it was asked to keep.
pub const DATABASE: &str = "/data/misc/keystore/persistent.sqlite";

/// The database copied with its log, into a folder that is not the phone's,
/// which the phone is not running from.
fn copy_with_log(database: &Path) -> Result<std::path::PathBuf> {
    let copy = std::path::PathBuf::from("/tmp/skyward-keystore.sqlite");
    std::fs::copy(database, &copy).with_context(|| format!("copy {}", database.display()))?;
    let log = std::path::PathBuf::from(format!("{}-wal", database.display()));
    if log.try_exists().unwrap_or(false) {
        let to = std::path::PathBuf::from(format!("{}-wal", copy.display()));
        std::fs::copy(&log, &to).with_context(|| format!("copy {}", log.display()))?;
    }
    Ok(copy)
}

/// The kind of thing a row holds: the key itself rather than a certificate.
const KEY_BLOB: i64 = 0;

/// The handle the secure world knows the key named `alias` by. The
/// database is copied first, with the log of what was written to it not
/// yet folded in: a phone writes to a database through that log, and a copy
/// opened as it lies would show none of what was written.
pub fn key_blob(database: &Path, alias: &str) -> Result<Vec<u8>> {
    let copy = copy_with_log(database)?;
    let db = rusqlite::Connection::open(&copy).with_context(|| format!("open {}", copy.display()))?;
    let mut query = db
        .prepare(
            "SELECT b.blob FROM blobentry b JOIN keyentry k ON b.keyentryid = k.id
             WHERE k.alias = ?1 AND b.subcomponent_type = ?2 ORDER BY b.id DESC LIMIT 1",
        )
        .context("ask the key service's store for a key")?;
    let mut rows = query.query((alias, KEY_BLOB)).context("ask the key service's store for a key")?;
    let row = rows
        .next()
        .with_context(|| format!("read what {} says about {alias}", database.display()))?
        .with_context(|| format!("{} holds no key named {alias}", database.display()))?;
    row.get(0).with_context(|| format!("read the key named {alias}"))
}
