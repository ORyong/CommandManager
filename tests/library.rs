use command_manager::{model::*, search, selection::Selection, store::Store};

fn command(name: &str, body: &str) -> Command {
    Command {
        id: new_id("cmd"),
        name: name.into(),
        command: body.into(),
        ..Default::default()
    }
}

#[test]
fn multi_group_is_one_command_and_group_removal_preserves_metadata() {
    let mut library = Library::default();
    library.add_group("Docker").unwrap();
    library.add_group("开发").unwrap();
    let ids: Vec<_> = library.groups.iter().map(|g| g.id.clone()).collect();
    let mut c = command("服务", "docker compose up");
    c.groups = ids.clone();
    c.notes = "开发环境".into();
    c.tags = vec!["本地".into()];
    library.save(c.clone()).unwrap();
    assert_eq!(library.commands.len(), 1);
    c.command = "docker compose up -d".into();
    library.save(c.clone()).unwrap();
    assert_eq!(library.commands[0].groups.len(), 2);
    assert_eq!(library.commands.len(), 1);
    library.remove_group(&ids[0]).unwrap();
    assert_eq!(library.commands[0].groups, vec![ids[1].clone()]);
    assert_eq!(library.commands[0].notes, "开发环境");
    assert_eq!(library.commands[0].tags, vec!["本地"]);
}

#[test]
fn tag_rename_and_delete_propagate_without_deleting_commands() {
    let mut library = Library::default();
    let mut c = command("测试", "echo test");
    c.tags = vec!["dev".into()];
    library.save(c).unwrap();
    let mut builtin = library
        .visible_commands()
        .into_iter()
        .find(Command::builtin)
        .unwrap();
    builtin.tags = vec!["dev".into()];
    library.save(builtin).unwrap();
    library.rename_tag("dev", "开发").unwrap();
    assert!(!library.tags.contains(&"dev".into()));
    assert!(
        library
            .visible_commands()
            .iter()
            .filter(|c| !c.tags.is_empty())
            .all(|c| c.tags == vec!["开发"])
    );
    library.remove_tag("开发");
    assert_eq!(library.commands.len(), 1);
    assert!(library.visible_commands().iter().all(|c| c.tags.is_empty()));
}

#[test]
fn legacy_import_converts_membership_and_builtin_ungrouped() {
    let bytes=br#"{"version":1,"commands":[{"id":"one","name":"Test","command":"echo ok","append_enter":true}],"organization":{"groups":[{"id":"g","name":"Group"}],"membership":{"one":"g","builtin:docker_logs":null}}}"#;
    let library = Library::decode(bytes).unwrap();
    assert_eq!(library.version, 2);
    assert_eq!(library.commands[0].groups, vec!["g"]);
    assert!(library.commands[0].tags.is_empty());
    assert!(library.commands[0].notes.is_empty());
    assert_eq!(
        library.builtin_metadata["builtin:docker_logs"].groups,
        Some(vec![])
    );
}

#[test]
fn roundtrip_preserves_unicode_and_multiline_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.json");
    let store = Store::new(path);
    let mut c = command("中文名字", "echo '一'\n  echo '二'\n");
    c.notes = "第一行\n第二行".into();
    c.tags = vec!["糖链".into(), "MD".into()];
    let saved = store.mutate(|library| library.save(c.clone())).unwrap();
    assert_eq!(store.read().unwrap(), saved);
    let out = dir.path().join("export.json");
    store.export(&out).unwrap();
    assert_eq!(
        Library::decode(&std::fs::read(out).unwrap()).unwrap(),
        saved
    );
    assert_eq!(saved.commands[0].command, c.command);
}

#[test]
fn stale_store_handles_reload_in_lock_and_preserve_previous_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.json");
    let a = Store::new(path.clone());
    let b = Store::new(path);
    a.mutate(|l| l.save(command("one", "echo one"))).unwrap();
    b.mutate(|l| l.save(command("two", "echo two"))).unwrap();
    assert_eq!(a.read().unwrap().commands.len(), 2);
}

#[test]
fn invalid_operation_or_corrupt_file_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.json");
    let store = Store::new(path.clone());
    store
        .mutate(|l| l.save(command("good", "echo good")))
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(store.mutate(|l| l.save(command("bad", ""))).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::write(&path, b"{invalid").unwrap();
    assert!(store.mutate(|l| l.save(command("x", "echo x"))).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{invalid");
}

#[test]
fn schema_rejects_duplicate_ids_dangling_groups_and_invalid_values() {
    let mut library = Library::default();
    let c = command("one", "echo one");
    library.commands = vec![c.clone(), c];
    assert!(library.validate().is_err());
    library.commands.truncate(1);
    library.commands[0].groups = vec!["missing".into()];
    assert!(library.validate().is_err());
    assert!(Library::decode(br#"{"version":9}"#).is_err());
    assert!(Library::decode(br#"{"version":1,"organization":[],"commands":[]}"#).is_err());
    assert!(Library::decode(br#"{"version":1,"organization":{"membership":{"builtin:one":"g"}},"builtin_metadata":[]}"#).is_err());
}

#[test]
fn builtin_body_is_authentic_and_metadata_survives_restore() {
    let mut library = Library::default();
    let mut c = builtin_templates()[0].clone();
    let id = c.id.clone();
    c.notes = "先检查参数".into();
    c.tags = vec!["安装".into()];
    library.save(c.clone()).unwrap();
    let mut changed = c.clone();
    changed.command = "evil".into();
    assert!(library.save(changed).is_err());
    library.remove(&[id.clone()]);
    assert!(!library.visible_commands().iter().any(|c| c.id == id));
    library.deleted_builtins.clear();
    assert_eq!(
        library
            .visible_commands()
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .notes,
        "先检查参数"
    );
}

#[test]
fn ordered_selection_reselects_at_end_and_joins_with_no_extra_final_newline() {
    let a = command("a", "one\ntwo\n");
    let b = command("b", "three\r\n");
    let mut selection = Selection::default();
    selection.toggle(&b.id);
    selection.toggle(&a.id);
    assert_eq!(
        selection.text(&[a.clone(), b.clone()]).unwrap(),
        "three\r\none\ntwo"
    );
    selection.toggle(&b.id);
    selection.toggle(&b.id);
    assert_eq!(selection.ordinal(&b.id), Some(2));
    assert_eq!(selection.text(&[a, b]).unwrap(), "one\ntwo\r\nthree");
}

#[test]
fn stale_selected_command_is_an_error_not_partial_copy() {
    let selection = Selection {
        ids: vec!["missing".into()],
    };
    assert!(selection.text(&[]).is_err());
}

#[test]
fn fuzzy_search_matches_all_fields_abbreviations_typos_and_chinese() {
    let mut c = command("Docker Compose", "docker compose up");
    c.notes = "创建环境".into();
    c.tags = vec!["部署".into(), "dev".into()];
    for query in [
        "DOCKER",
        "dck cmp",
        "部署 创建",
        "dockre",
        "创建坏境",
        "dck 创建坏境",
    ] {
        assert_eq!(
            search::filter(&[c.clone()], query, &[], false).len(),
            1,
            "query {query}"
        );
    }
    assert!(search::filter(&[c.clone()], "notexisting", &[], false).is_empty());
    assert!(search::filter(&[c], &"x".repeat(2049), &[], false).is_empty());
}

#[test]
fn tags_all_any_and_query_combine() {
    let mut a = command("one", "docker");
    a.tags = vec!["A".into(), "B".into()];
    let mut b = command("two", "docker");
    b.tags = vec!["A".into()];
    let commands = vec![a, b];
    let tags = vec!["a".into(), "B".into()];
    assert_eq!(search::filter(&commands, "docker", &tags, false).len(), 1);
    assert_eq!(search::filter(&commands, "docker", &tags, true).len(), 2);
    assert!(search::filter(&commands, "missing", &tags, true).is_empty());
}

#[test]
fn concurrent_writers_preserve_all_committed_commands() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("concurrent.json");
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|index| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let store = Store::new(path);
                let command = command(&format!("writer-{index}"), "echo test");
                barrier.wait();
                for _ in 0..1000 {
                    if store
                        .mutate(|library| library.save(command.clone()))
                        .is_ok()
                    {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                panic!("writer never acquired transaction lock");
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(Store::new(path).read().unwrap().commands.len(), 8);
}

#[test]
fn tag_case_variants_are_renamed_and_removed_together() {
    let mut library = Library::default();
    let mut a = command("A", "echo a");
    a.tags = vec!["Dev".into()];
    let mut b = command("B", "echo b");
    b.tags = vec!["dev".into()];
    library.save(a).unwrap();
    library.save(b).unwrap();
    library.rename_tag("Dev", "DEV").unwrap();
    assert!(
        library
            .commands
            .iter()
            .all(|command| command.tags == vec!["DEV"])
    );
    library.remove_tag("dev");
    assert!(
        library
            .commands
            .iter()
            .all(|command| command.tags.is_empty())
    );
}
