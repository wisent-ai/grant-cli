//! The built grant-cli against the fleet database it resolves through Stado:
//! an organization is created, read back, edited, deleted, and then refused
//! as absent. It needs a host where `stado database resolve grant-cli
//! --consumer grant-cli` answers and the grant-cli-database-client bearer is
//! synced; without them the first command fails and says which step did.
//! The organization's slug is unique to the run, and the run deletes it.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn home() -> PathBuf {
    let home = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("grant-cli-db-lifecycle");
    std::fs::create_dir_all(&home).expect("create the run's object directory");
    home
}

fn grant(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grant-cli"))
        .arg("--json")
        .arg("--home")
        .arg(home())
        .args(arguments)
        .output()
        .expect("start the built grant-cli")
}

fn answered(arguments: &[&str]) -> Value {
    let output = grant(arguments);
    assert!(
        output.status.success(),
        "grant {} exited {}: {}",
        arguments.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("grant answers JSON")
}

fn refused(arguments: &[&str]) -> String {
    let output = grant(arguments);
    assert!(!output.status.success(), "grant {} should have been refused", arguments.join(" "));
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn organization_lives_in_the_fleet_database() {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).expect("clock after 1970").as_nanos();
    let slug = format!("lifecycle-{}-{nanos}", std::process::id());

    let created = answered(&["organization", "set", &slug, "--name", "Lifecycle", "--profile", r#"{"country":"PL"}"#]);
    assert_eq!(created["slug"], slug.as_str());

    let shown = answered(&["organization", "show", &slug]);
    assert_eq!(shown["id"], created["id"]);
    assert_eq!(shown["profile"]["country"], "PL");

    answered(&["organization", "set", &slug, "--name", "Lifecycle renamed", "--profile", r#"{"country":"DE"}"#]);
    let edited = answered(&["organization", "show", &slug]);
    assert_eq!(edited["id"], created["id"]);
    assert_eq!(edited["name"], "Lifecycle renamed");
    assert_eq!(edited["profile"]["country"], "DE");

    let deleted = answered(&["organization", "delete", &slug]);
    assert_eq!(deleted["id"], created["id"]);

    assert!(refused(&["organization", "show", &slug]).contains("organization not found"));
    assert!(refused(&["organization", "delete", &slug]).contains("organization not found"));
}
