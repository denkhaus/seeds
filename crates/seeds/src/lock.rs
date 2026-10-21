//! The `.seeds/` writer lock: advisory, exclusive, crash-safe.
//!
//! Every mutating command wraps its whole load→mutate→save cycle in
//! [`WriterLock`] (seeds-0d7b): an advisory exclusive lock on
//! `.seeds/.lock`, taken via [`std::fs::File::try_lock`] (flock
//! semantics on Unix). Two seeds processes can therefore never
//! silently drop each other's records — the second writer waits, and
//! fails fast once the timeout passes.
//!
//! The lock is deliberately kernel-owned: it lives on the open file
//! description, so the kernel releases it when the holder exits,
//! crashed or not — a dead writer cannot wedge the tracker, and there
//! is no stale-lockfile recovery machinery. The lock file is a
//! permanent zero-byte `.seeds/` sibling that `init`'s `.gitignore`
//! (`*.lock`) keeps out of version control.

use std::fs::OpenOptions;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::error::Error;

/// How long a writer waits for the lock before failing fast.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Poll interval while waiting for the lock.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// The advisory writer lock on `.seeds/.lock`.
///
/// Dropping the guard closes the file description and releases the
/// lock; process exit does the same through the kernel.
#[derive(Debug)]
pub struct WriterLock {
    /// Held for its open file description — the kernel owns the lock
    /// and releases it when the description closes (drop or process
    /// exit). Never read or written.
    #[expect(
        dead_code,
        reason = "the file is the lock; its contents are irrelevant"
    )]
    file: std::fs::File,
}

impl WriterLock {
    /// Acquires the writer lock at `root/.lock`, waiting up to
    /// [`ACQUIRE_TIMEOUT`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::LockTimeout`] when another process still holds
    /// the lock past the timeout, and [`Error::Io`] on lock-file or
    /// locking failures.
    pub fn acquire(root: &Path) -> Result<Self, Error> {
        Self::acquire_with_timeout(root, ACQUIRE_TIMEOUT)
    }

    /// Probes whether ANOTHER file description holds the writer lock
    /// at `root/.lock` (seeds-36cd): `doctor`'s report-only mode reads
    /// the store while a write may be mid-cycle, so it names the
    /// concurrent writer instead of pretending the read was isolated.
    ///
    /// The probe only OPENS an existing lock file — it never creates
    /// one, never waits, and releases whatever it took by the time it
    /// returns (the file description drops with the local `file`).
    /// A `doctor --fix` run holds the lock itself, so its caller must
    /// not probe.
    #[must_use]
    pub fn is_held_elsewhere(root: &Path) -> bool {
        let path = root.join(".lock");
        let Ok(file) = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
        else {
            // No lock file: no writer has ever held it.
            return false;
        };
        match file.try_lock() {
            // Taken by the probe: dropping `file` before returning
            // releases it. An OS error counts as "not held" — the
            // caller reports only what it can actually observe.
            Ok(()) | Err(std::fs::TryLockError::Error(_)) => false,
            Err(std::fs::TryLockError::WouldBlock) => true,
        }
    }

    /// Acquires the writer lock with an explicit wait budget.
    ///
    /// # Errors
    ///
    /// Returns [`Error::LockTimeout`] when another process still holds
    /// the lock past `timeout`, and [`Error::Io`] on lock-file or
    /// locking failures.
    pub fn acquire_with_timeout(root: &Path, timeout: Duration) -> Result<Self, Error> {
        let path = root.join(".lock");
        // create + !truncate: the lock file is a permanent zero-byte
        // sibling; opening must never clobber an existing one.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
        let deadline = Instant::now() + timeout;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Self { file }),
                Err(std::fs::TryLockError::WouldBlock) => {}
                Err(std::fs::TryLockError::Error(source)) => {
                    return Err(Error::Io { path, source });
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::LockTimeout { path });
            }
            std::thread::sleep(POLL_INTERVAL.min(remaining));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::mpsc;

    use super::*;

    fn temp_seeds_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("seeds-lock-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp .seeds dir");
        dir
    }

    #[test]
    fn second_writer_times_out_while_first_holds() {
        let dir = temp_seeds_dir("timeout");
        let first = WriterLock::acquire(&dir).expect("first writer acquires");
        let error = WriterLock::acquire_with_timeout(&dir, Duration::from_millis(100))
            .expect_err("second writer must not acquire");
        assert!(
            matches!(error, Error::LockTimeout { .. }),
            "expected LockTimeout, got {error:?}"
        );
        drop(first);
    }

    #[test]
    fn lock_releases_on_drop() {
        let dir = temp_seeds_dir("release");
        drop(WriterLock::acquire(&dir).expect("first writer acquires"));
        WriterLock::acquire_with_timeout(&dir, Duration::from_millis(100))
            .expect("reacquires after drop");
    }

    /// The doctor probe (seeds-36cd) names an active holder and stays
    /// quiet otherwise — including before the lock file ever exists.
    #[test]
    fn probe_reports_an_active_holder() {
        let dir = temp_seeds_dir("probe");
        assert!(
            !WriterLock::is_held_elsewhere(&dir),
            "no lock file: nobody can hold it"
        );
        let held = WriterLock::acquire(&dir).expect("first writer acquires");
        assert!(
            WriterLock::is_held_elsewhere(&dir),
            "the held lock is visible to a second description"
        );
        drop(held);
        assert!(
            !WriterLock::is_held_elsewhere(&dir),
            "released: the probe reads it as free"
        );
    }

    #[test]
    fn waiting_writer_acquires_after_release() {
        let dir = temp_seeds_dir("handoff");
        let first = WriterLock::acquire(&dir).expect("first writer acquires");
        let (held_tx, held_rx) = mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let _second =
                WriterLock::acquire_with_timeout(&dir, Duration::from_secs(5)).expect("waits");
            held_tx.send(()).expect("sender alive");
        });
        // While `first` holds the lock, the waiter must not get it.
        assert!(held_rx.recv_timeout(Duration::from_millis(150)).is_err());
        drop(first);
        held_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("waiter acquired after release");
        waiter.join().expect("waiter thread");
    }
}
