use std::fs;

use anyhow::bail;
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
