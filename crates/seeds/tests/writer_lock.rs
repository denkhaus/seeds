//! Writer-lock behavior (seeds-0d7b) at the CLI level: parallel
//! writers keep every record, a writer survives another holder's
//! death without wedging, and read commands never create
//! `.seeds/.lock`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_store(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "seeds-writer-lock-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed),
    ));
    let seeds = dir.join(".seeds");
    fs::create_dir_all(&seeds).expect("temp .seeds dir");
    fs::write(
        seeds.join("config.yaml"),
        "project: \"tst\"\nversion: \"1\"\nmax_plan_depth: 3\n",
    )
    .expect("config.yaml");
    fs::write(seeds.join("issues.jsonl"), "").expect("issues.jsonl");
    fs::write(seeds.join("plans.jsonl"), "").expect("plans.jsonl");
    fs::write(seeds.join("templates.jsonl"), "").expect("templates.jsonl");
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seeds"))
        .args(args)
        .current_dir(dir)
        .env_remove("RUST_BACKTRACE")
        .output()
        .expect("binary runs")
}

/// Eight writers race on one store; every record must survive.
#[test]
fn parallel_creates_keep_every_record() {
    let dir = temp_store("parallel");
    let mut children: Vec<Child> = (0..8)
        .map(|i| {
            Command::new(env!("CARGO_BIN_EXE_seeds"))
                .args(["create", "--title", &format!("racer {i}")])
                .current_dir(&dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("writer spawns")
        })
        .collect();
    for child in &mut children {
        let status = child.wait().expect("writer finishes");
        assert!(status.success(), "writer failed: {status:?}");
    }

    let output = run(&dir, &["list", "--format", "ids"]);
    assert!(output.status.success(), "list failed: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("ids are UTF-8");
    let ids: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(ids.len(), 8, "lost records: {stdout}");
    assert_eq!(unique.len(), ids.len(), "duplicate ids: {stdout}");
}

/// Read commands stay lock-free: they never create `.seeds/.lock`;
/// a mutating command does.
#[test]
fn reads_never_create_the_lock_file() {
    let dir = temp_store("read-clean");
    for args in [
        &["list"][..],
        &["stats"][..],
        &["ready"][..],
        &["doctor"][..],
    ] {
        let output = run(&dir, args);
        assert!(output.status.success(), "{args:?} failed: {output:?}");
    }
    assert!(
        !dir.join(".seeds/.lock").exists(),
        "a read command created the lock file"
    );

    let output = run(&dir, &["create", "--title", "writes"]);
    assert!(output.status.success(), "create failed: {output:?}");
    assert!(
        dir.join(".seeds/.lock").exists(),
        "create left no lock file behind"
    );
}

/// A writer whose predecessor dies mid-hold proceeds immediately:
/// the kernel released the dead holder's lock.
#[test]
fn create_proceeds_after_holder_dies() {
    let dir = temp_store("crash-release");
    let mut holder = Command::new(std::env::current_exe().expect("test exe"))
        .args(["--exact", "writer_lock_hold_helper", "--nocapture"])
        .env("SEEDS_TEST_LOCK_DIR", &dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("holder spawns");

    // Wait until the holder announces it holds the lock.
    let mut stdout = holder.stdout.take().expect("holder stdout");
    let mut held = false;
    for line in BufReader::new(&mut stdout).lines() {
        if line.expect("holder line") == "held" {
            held = true;
            break;
        }
    }
    assert!(held, "holder never announced the lock");
    drop(stdout);

    // The writer starts, waits on the lock, then the holder dies.
    let started = Instant::now();
    let mut writer = Command::new(env!("CARGO_BIN_EXE_seeds"))
        .args(["create", "--title", "after crash"])
        .current_dir(&dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("writer spawns");
    std::thread::sleep(Duration::from_millis(200));
    holder.kill().expect("holder killed");
    holder.wait().expect("holder reaped");

    let status = writer.wait().expect("writer finishes");
    assert!(
        status.success(),
        "writer failed after holder death: {status:?}"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(150),
        "writer did not wait for the held lock"
    );

    let output = run(&dir, &["list", "--format", "ids"]);
    let stdout = String::from_utf8(output.stdout).expect("ids are UTF-8");
    assert_eq!(
        stdout.lines().filter(|l| !l.trim().is_empty()).count(),
        1,
        "record missing: {stdout}"
    );
}

/// Helper process for [`create_proceeds_after_holder_dies`]: holds an
/// exclusive lock on `.seeds/.lock` until killed. A plain suite run
/// (no `SEEDS_TEST_LOCK_DIR`) returns immediately.
#[test]
#[expect(
    clippy::print_stdout,
    reason = "the held marker is the parent test's synchronization signal"
)]
fn writer_lock_hold_helper() {
    let Some(dir) = std::env::var_os("SEEDS_TEST_LOCK_DIR") else {
        return;
    };
    let path = Path::new(&dir).join(".seeds/.lock");
    // create + !truncate: never clobber an existing lock file.
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .expect("lock file opens");
    file.try_lock().expect("helper acquires");
    println!("held");
    std::thread::sleep(Duration::from_secs(30));
}
