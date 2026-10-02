use std::collections::BTreeMap;
use std::fs;

use anyhow::bail;
use herdr_cadence::model::ProjectState;
use herdr_cadence::state::StateStore;

#[test]
fn failed_state_updates_do_not_persist_partial_changes_in_special_paths() {
    let temp = tempfile::tempdir().unwrap();
    let state_dir = temp.path().join("state with spaces; $HOME");
    let state = StateStore::new(&state_dir);

    state
        .update(|store| {
            store.schema_version = 1;
            Ok(())
        })
        .unwrap();
    let before = fs::read(state_dir.join("state.json")).unwrap();

    let error = state
        .update(|store| -> anyhow::Result<()> {
            store.schema_version = 999;
            bail!("simulated update failure")
        })
        .unwrap_err();

    assert!(error.to_string().contains("simulated update failure"));
    assert_eq!(fs::read(state_dir.join("state.json")).unwrap(), before);
    assert_eq!(state.read().unwrap().schema_version, 1);
}

#[test]
fn large_state_round_trips_and_can_be_replaced_with_smaller_state() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateStore::new(temp.path());
    let expected = state
        .update(|store| {
            for number in 0..128 {
                store.projects.insert(
                    format!("project-{number}"),
                    ProjectState {
                        root: format!(
                            "/fixture/{number}/{}",
                            "path with spaces/λ/\"quoted\"/".repeat(64)
                        ),
                        active_run: None,
                        runs: BTreeMap::new(),
                    },
                );
            }
            Ok(store.clone())
        })
        .unwrap();
    let mut expected_bytes = serde_json::to_vec_pretty(&expected).unwrap();
    expected_bytes.push(b'\n');
    assert!(expected_bytes.len() > 8192);
    assert_eq!(
        fs::read(temp.path().join("state.json")).unwrap(),
        expected_bytes
    );
    assert_eq!(
        serde_json::to_value(state.read().unwrap()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );

    state
        .update(|store| {
            store.projects.clear();
            Ok(())
        })
        .unwrap();
    let loaded = state.read().unwrap();
    assert!(loaded.projects.is_empty());
    let mut expected_bytes = serde_json::to_vec_pretty(&loaded).unwrap();
    expected_bytes.push(b'\n');
    assert_eq!(
        fs::read(temp.path().join("state.json")).unwrap(),
        expected_bytes
    );
}
