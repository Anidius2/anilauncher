mod auth;
mod backup;
mod conflicts;
mod content;
mod crash;
mod defaults;
mod gameui;
mod discord;
mod forge;
mod friend;
mod instances;
mod launch;
mod link;
mod media;
mod net;
mod own;
mod news;
mod packs;
mod quick;
mod report;
mod skins;
mod snapshots;
mod stats;
mod state;
mod tray;
mod updates;
mod util;
mod versions;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        // a second start (e.g. from the desktop while hidden in the tray) brings back this window
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_window(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--tray"])))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().pubkey(updates::PUBKEY).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let state = state::AppState::new(app.handle());
            app.manage(state);
            app.manage(updates::Pending::default());
            if let Err(e) = tray::create(app.handle()) {
                eprintln!("{e}");
            }
            // started with Windows: stay in the tray
            if std::env::args().any(|a| a == "--tray") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            state::get_settings,
            discord::discord_presence,
            updates::check_update,
            updates::install_update,
            updates::updates_configured,
            tray::tray_update,
            tray::hide_to_tray,
            tray::set_autostart,
            backup::estimate_launcher_backup,
            backup::create_launcher_backup,
            backup::inspect_launcher_backup,
            backup::restore_launcher_backup,
            report::bug_report,
            report::reveal_file,
            link::link_send,
            link::link_status,
            link::link_enable_pack,
            link::screenshot_meta,
            link::world_times,
            state::save_settings,
            state::import_media,
            state::open_url,
            auth::list_accounts,
            auth::login_start,
            auth::login_poll,
            auth::login_window,
            defaults::apply_new_build_defaults,
            defaults::apply_defaults_to_all,
            auth::name_available,
            auth::set_minecraft_name,
            auth::add_offline,
            auth::set_active_account,
            auth::remove_account,
            versions::list_game_versions,
            versions::list_loader_versions,
            instances::list_instances,
            instances::create_instance,
            instances::update_instance,
            instances::delete_instance,
            instances::open_instance_folder,
            instances::duplicate_instance,
            instances::read_launch_log,
            state::system_memory,
            state::open_file,
            launch::launch_instance,
            launch::running_instances,
            launch::stop_instance,
            launch::launch_bedrock,
            news::get_feed,
            content::modrinth_search,
            content::modrinth_versions,
            content::install_content,
            content::list_content,
            content::toggle_content,
            content::remove_content,
            content::check_updates,
            content::apply_updates,
            content::rollback_content,
            content::pin_content,
            packs::import_pack,
            packs::export_pack,
            packs::install_modpack,
            content::optimize_instance,
            conflicts::check_mods,
            crash::crash_fix,
            snapshots::list_snapshots,
            snapshots::create_snapshot,
            snapshots::restore_snapshot,
            snapshots::delete_snapshot,
            stats::play_sessions,
            stats::minecraft_latest,
            quick::quick_start,
            quick::list_controls,
            quick::save_controls,
            quick::apply_controls,
            quick::delete_controls,
            friend::resolve_share,
            friend::analyze_drop,
            friend::place_drop,
            friend::save_png,
            friend::copy_png,
            forge::list_forge_versions,
            media::list_screenshots,
            media::copy_image,
            media::save_copy,
            media::delete_screenshot,
            media::list_worlds,
            media::list_backups,
            media::backup_world,
            media::restore_backup,
            media::delete_backup,
            skins::list_skins,
            skins::import_skin,
            skins::save_skin_data,
            skins::update_skin,
            skins::delete_skin,
            skins::texture_data,
            skins::skin_profile,
            skins::upload_skin,
            skins::set_cape,
            skins::open_namemc,
            skins::read_png,
            skins::write_png,
            state::read_text_file,
            state::write_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AniLauncher");
}

#[cfg(test)]
mod live_tests {
    //! Network tests: `cargo test -- --ignored --nocapture`
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    #[ignore]
    async fn install_and_start_fabric() {
        let mc = std::env::var("TEST_MC").unwrap_or_else(|_| "1.21.11".into());
        let loader = std::env::var("TEST_LOADER").unwrap_or_else(|_| "fabric".into());
        let root = std::env::temp_dir().join("anilauncher-test");
        let st = state::AppState::with_root(root.clone());
        let emit: versions::Emit = Arc::new(|stage: &str, d: usize, t: usize| {
            if d == t {
                println!("{stage}: {d}/{t}");
            }
        });
        let game_dir = root.join("instances/test/minecraft");
        std::fs::create_dir_all(&game_dir).unwrap();
        let p = versions::prepare(&st, &emit, &mc, &loader, None, &game_dir).await.expect("prepare");
        let inst = instances::Instance {
            id: "test".into(),
            name: "Test".into(),
            mc_version: mc.clone(),
            loader: loader.clone(),
            memory_mb: Some(2048),
            ..Default::default()
        };
        let acc = auth::Account {
            id: "x".into(),
            name: "Tester".into(),
            kind: "offline".into(),
            uuid: util::offline_uuid("Tester"),
            ..Default::default()
        };
        let args = launch::build_args(&st, &inst, &acc, &p, &game_dir, None).expect("args");
        println!("java = {}", p.java.display());
        println!("main = {}", p.json["mainClass"]);
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            tokio::process::Command::new(&p.java).args(&args).current_dir(&game_dir).kill_on_drop(true).output(),
        )
        .await;
        match out {
            Ok(Ok(o)) => {
                let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                let tail: String = text.lines().rev().take(25).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                println!("--- game output (tail) ---\n{tail}");
            }
            Ok(Err(e)) => panic!("spawn failed: {e}"),
            Err(_) => println!("game still running after 60s (good sign)"),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn link_in_real_game() {
        use serde_json::json;
        use tauri::Manager;
        let mc = std::env::var("TEST_MC").unwrap_or_else(|_| "1.21.11".into());
        let loader = std::env::var("TEST_LOADER").unwrap_or_else(|_| "fabric".into());
        let root = std::env::temp_dir().join("anilauncher-test");
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let emit: versions::Emit = Arc::new(|_: &str, _: usize, _: usize| {});
        let id = format!("link{}{}", loader, mc.replace('.', ""));
        let game_dir = instances::game_dir(&st, &id);
        let _ = std::fs::remove_dir_all(game_dir.join("screenshots"));
        let _ = std::fs::remove_file(root.join("media/screenshot-meta.json"));
        std::fs::create_dir_all(game_dir.join("resourcepacks")).unwrap();
        // a tiny resource pack to switch on from the launcher
        {
            let f = std::fs::File::create(game_dir.join("resourcepacks/AniTest.zip")).unwrap();
            let mut z = zip::ZipWriter::new(f);
            z.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default()).unwrap();
            use std::io::Write;
            z.write_all(br#"{"pack":{"pack_format":46,"min_format":1,"max_format":200,"supported_formats":[1,200],"description":"AniLink test"}}"#).unwrap();
            z.finish().unwrap();
        }
        std::fs::write(game_dir.join("options.txt"), "onboardAccessibility:false\nskipMultiplayerWarning:true\njoinedFirstServer:true\n").unwrap();
        let p = versions::prepare(&st, &emit, &mc, &loader, None, &game_dir).await.expect("prepare");
        let inst = instances::Instance { id: id.clone(), name: "Link".into(), mc_version: mc.clone(), loader: loader.clone(), memory_mb: Some(2048), ..Default::default() };
        let acc = auth::Account { id: "x".into(), name: "Tester".into(), kind: "offline".into(), uuid: util::offline_uuid("Tester"), ..Default::default() };
        let mut args = launch::build_args(&st, &inst, &acc, &p, &game_dir, None).expect("args");
        let extra = link::jvm_args(app.handle(), &st, &id, &p.mc, &loader, &p.json).unwrap().expect("link args");
        println!("link args: {extra:?}");
        extra.apply(&mut args);
        let mut child = tokio::process::Command::new(&p.java).args(&args).current_dir(&game_dir)
            .env("DISPLAY", ":99").env("LIBGL_ALWAYS_SOFTWARE", "1")
            .stdout(std::fs::File::create(game_dir.join("stdout.txt")).unwrap()).stderr(std::fs::File::create(game_dir.join("stderr.txt")).unwrap()).kill_on_drop(true).spawn().unwrap();
        let hub = st.link.clone();
        let t0 = std::time::Instant::now();
        while !hub.status().contains_key(&id) {
            assert!(t0.elapsed().as_secs() < 300, "game never connected");
            if let Ok(Some(code)) = child.try_wait() { panic!("game exited early: {code}"); }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        println!("connected after {:?}", t0.elapsed());
        // let the title screen settle
        tokio::time::sleep(std::time::Duration::from_secs(25)).await;
        let r = hub.send(&id, json!({"cmd":"getOptions"})).await.unwrap();
        println!("options: {}", r["options"]);
        hub.send(&id, json!({"cmd":"setOptions","values":{"fov":95,"guiScale":2,"gamma":1.0}})).await.unwrap();
        let r = hub.send(&id, json!({"cmd":"getOptions"})).await.unwrap();
        println!("options after: {}", r["options"]);
        assert_eq!(r["options"]["fov"], 95);
        let r = hub.send(&id, json!({"cmd":"listPacks"})).await.unwrap();
        println!("packs: {}", r["packs"]);
        hub.send(&id, json!({"cmd":"enablePack","pack":"file/AniTest.zip"})).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        let r = hub.send(&id, json!({"cmd":"listPacks"})).await.unwrap();
        println!("packs after: {}", r["packs"]["selected"]);
        assert!(r["packs"]["selected"].to_string().contains("AniTest"));
        hub.send(&id, json!({"cmd":"setKeys","keys":{"key_key.jump":"key.keyboard.j"}})).await.unwrap();
        let opts = std::fs::read_to_string(game_dir.join("options.txt")).unwrap();
        assert!(opts.contains("key_key.jump:key.keyboard.j"), "key not saved");
        let bad = hub.send(&id, json!({"cmd":"nope"})).await;
        println!("unknown command -> {bad:?}");
        hub.send(&id, json!({"cmd":"toast","title":"AniLauncher","text":"Привет из лаунчера!"})).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let _ = std::process::Command::new("xdotool").env("DISPLAY", ":99").args(["search", "--name", "Minecraft", "windowfocus", "--sync", "%1", "key", "F2"]).status();
        let meta_path = root.join("media/screenshot-meta.json");
        let t1 = std::time::Instant::now();
        loop {
            if std::fs::read_to_string(&meta_path).map(|s| s.contains(&id)).unwrap_or(false) { break; }
            assert!(t1.elapsed().as_secs() < 20, "screenshot not reported");
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        println!("screenshot meta: {}", std::fs::read_to_string(&meta_path).unwrap());
        let shots: Vec<_> = std::fs::read_dir(game_dir.join("screenshots")).unwrap().flatten().map(|e| e.path()).collect();
        println!("SHOT {}", shots[0].display());
        if let Ok(hold) = std::env::var("TEST_HOLD") {
            // keep the game open to drive it by hand (enter a world etc.)
            let secs: u64 = hold.parse().unwrap_or(120);
            for _ in 0..secs {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if let Ok(Some(_)) = child.try_wait() { break; }
            }
            println!("status at end: {:?}", hub.status());
            println!("world time: {:?}", std::fs::read_to_string(root.join(format!("instances/{id}/launcher/world-time.json"))));
        }
        let _ = child.kill().await;
    }

    #[tokio::test]
    #[ignore]
    async fn backup_and_report_on_real_data() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-test");
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        {
            let st = app.state::<state::AppState>();
            for (id, mc, loader) in [("link12111", "1.21.11", "fabric"), ("linkforge1216", "1.21.6", "forge")] {
                if !instances::dir(&st, id).join("instance.json").exists() {
                    let inst = instances::Instance { id: id.into(), name: format!("Тест {loader}"), mc_version: mc.into(), loader: loader.into(), ..Default::default() };
                    instances::store(&st, &inst).unwrap();
                }
            }
        }
        let opts: backup::BackupOptions = serde_json::from_value(serde_json::json!({"worlds": true, "history": true, "accounts": false})).unwrap();
        let est = backup::estimate_launcher_backup(app.state(), opts);
        println!("estimate: {}", serde_json::to_string(&est).unwrap());
        let dest = std::env::temp_dir().join("ani-backup.zip");
        let bytes = backup::create_launcher_backup(app.handle().clone(), app.state(), dest.to_string_lossy().into(), opts).await.unwrap();
        println!("backup: {} MB raw, zip {} MB", bytes / 1_000_000, std::fs::metadata(&dest).unwrap().len() / 1_000_000);
        let info = backup::inspect_launcher_backup(dest.to_string_lossy().into()).unwrap();
        println!("info: {}", serde_json::to_string(&info).unwrap());
        // restore into an empty launcher folder
        let fresh = std::env::temp_dir().join("anilauncher-restored");
        let _ = std::fs::remove_dir_all(&fresh);
        let app2 = tauri::test::mock_app();
        app2.manage(state::AppState::with_root(fresh.clone()));
        let n = backup::restore_launcher_backup(app2.handle().clone(), app2.state(), dest.to_string_lossy().into()).await.unwrap();
        println!("restored {n} files");
        let st2 = app2.state::<state::AppState>();
        let ids: Vec<String> = std::fs::read_dir(st2.instances()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        println!("builds after restore: {}", ids.len());
        assert!(!fresh.join("accounts.json").exists(), "accounts must stay out");
        assert!(fresh.join("instances/link12111/minecraft/saves").exists(), "worlds restored");
        assert!(!fresh.join("instances/link12111/minecraft/logs").exists(), "logs skipped");
        // bug report for one build
        let rep = std::env::temp_dir().join("ani-report.zip");
        report::bug_report(app.state(), rep.to_string_lossy().into(), Some("linkforge1216".into()), Some("Игра вылетает при входе в мир".into())).await.unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&rep).unwrap()).unwrap();
        let names: Vec<String> = (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect();
        println!("report files: {names:?}");
        let mut all = String::new();
        for i in 0..z.len() { use std::io::Read; let mut s = String::new(); let _ = z.by_index(i).unwrap().read_to_string(&mut s); all.push_str(&s); }
        assert!(!all.contains("anilink.token=") || all.contains("anilink.token= <скрыто>") || !all.contains("-Danilink.token=a"), "token leaked");
        let mut readme = String::new();
        { use std::io::Read; z.by_name("README.txt").unwrap().read_to_string(&mut readme).unwrap(); }
        println!("--- README ---\n{}", readme.lines().take(14).collect::<Vec<_>>().join("\n"));
        let _ = std::fs::remove_dir_all(&fresh);
    }

    #[tokio::test]
    #[ignore]
    async fn modrinth_install_update_export_import() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-content-test");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let inst = instances::Instance {
            id: "t1".into(), name: "Mods".into(), mc_version: "1.21.1".into(), loader: "fabric".into(),
            ..Default::default()
        };
        std::fs::create_dir_all(instances::game_dir(&st, "t1")).unwrap();
        instances::store(&st, &inst).unwrap();

        let found = content::modrinth_search(app.state(), "mod menu".into(), "mod".into(), Some("1.21.1".into()), Some("fabric".into()), 0, None).await.unwrap();
        println!("search ok");
        let _ = found;
        let titles = content::install_content(app.state(), "t1".into(), "mod".into(), "modmenu".into(), None).await.unwrap();
        println!("installed: {titles:?}");
        // put an old Sodium version to test updates
        let versions = content::modrinth_versions(app.state(), "t1".into(), "sodium".into(), "mod".into()).await.unwrap();
        let old = serde_json::to_value(&versions).unwrap();
        let old_id = old.as_array().unwrap().last().unwrap()["id"].as_str().unwrap().to_string();
        content::install_content(app.state(), "t1".into(), "mod".into(), "sodium".into(), Some(old_id)).await.unwrap();
        let list = content::list_content(app.state(), "t1".into(), "mod".into()).await.unwrap();
        println!("list: {}", serde_json::to_string(&list).unwrap().chars().take(400).collect::<String>());
        let ups = content::check_updates(app.state(), "t1".into()).await.unwrap();
        println!("updates: {}", serde_json::to_string(&ups).unwrap());
        let n = content::apply_updates(app.state(), "t1".into(), vec![]).await.unwrap();
        println!("applied {n}");
        let ups2 = content::check_updates(app.state(), "t1".into()).await.unwrap();
        println!("updates after: {}", ups2.len());
        // one mod steps back; it is pinned, so it no longer shows up as an update
        let list = content::list_content(app.state(), "t1".into(), "mod".into()).await.unwrap();
        let sod = list.iter().find(|c| serde_json::to_value(c).unwrap()["previous"].is_string()).expect("updated mod remembers its previous version");
        let sod = serde_json::to_value(sod).unwrap();
        println!("previous: {} {}", sod["filename"], sod["previous"]);
        let back = content::rollback_content(app.state(), "t1".into(), "mod".into(), sod["filename"].as_str().unwrap().into()).await.unwrap();
        println!("rolled back to {back}");
        let ups3 = content::check_updates(app.state(), "t1".into()).await.unwrap();
        assert!(ups3.is_empty(), "a rolled-back mod is not offered again");
        assert!(serde_json::to_string(&ups).unwrap().contains("\"changelog\":\""));
        let dest = root.join("export.mrpack");
        packs::export_pack(app.state(), "t1".into(), dest.to_string_lossy().into()).await.unwrap();
        println!("exported {} bytes", std::fs::metadata(&dest).unwrap().len());
        let imported = packs::import_pack(app.handle().clone(), app.state(), dest.to_string_lossy().into()).await.unwrap();
        let mods: Vec<_> = std::fs::read_dir(instances::game_dir(&st, &imported.id).join("mods")).unwrap().flatten().map(|e| e.file_name()).collect();
        println!("imported {} {} {:?} mods={mods:?}", imported.name, imported.loader, imported.loader_version);
    }

    #[tokio::test]
    async fn world_backup_and_restore() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-backup-test");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let world = instances::game_dir(&st, "w1").join("saves").join("My World");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), b"one").unwrap();
        std::fs::write(world.join("region/r.0.0.mca"), b"blocks").unwrap();
        let file = media::backup_world(app.state(), "w1".into(), "My World".into()).await.unwrap();
        assert_eq!(media::list_worlds(app.state(), "w1".into()).len(), 1);
        std::fs::write(world.join("level.dat"), b"two").unwrap();
        media::restore_backup(app.state(), "w1".into(), file.clone()).await.unwrap();
        assert_eq!(std::fs::read(world.join("level.dat")).unwrap(), b"one");
        assert_eq!(std::fs::read(world.join("region/r.0.0.mca")).unwrap(), b"blocks");
        assert!(media::list_backups(app.state(), "w1".into()).len() >= 1);
    }

    #[tokio::test]
    #[ignore]
    async fn modpack_and_optimize() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-pack-test");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let inst = packs::install_modpack(app.handle().clone(), app.state(), "fabulously-optimized".into(), None).await.unwrap();
        let mods = std::fs::read_dir(instances::game_dir(&st, &inst.id).join("mods")).map(|r| r.count()).unwrap_or(0);
        println!("modpack: {} {} {} {:?} mods={mods} art={:?}", inst.name, inst.mc_version, inst.loader, inst.loader_version, inst.art.as_ref().map(|a| &a.value));
        let fresh = instances::Instance { id: "opt".into(), name: "Opt".into(), mc_version: "1.21.1".into(), loader: "fabric".into(), ..Default::default() };
        std::fs::create_dir_all(instances::game_dir(&st, "opt")).unwrap();
        instances::store(&st, &fresh).unwrap();
        let r = content::optimize_instance(app.state(), "opt".into()).await.unwrap();
        println!("optimize: {}", serde_json::to_string(&r).unwrap());
        let again = content::optimize_instance(app.state(), "opt".into()).await.unwrap();
        println!("again: {}", serde_json::to_string(&again).unwrap());
        let neo = instances::Instance { id: "neo".into(), name: "Neo".into(), mc_version: "1.21.1".into(), loader: "neoforge".into(), ..Default::default() };
        std::fs::create_dir_all(instances::game_dir(&st, "neo")).unwrap();
        instances::store(&st, &neo).unwrap();
        let r = content::optimize_instance(app.state(), "neo".into()).await.unwrap();
        println!("neoforge optimize: {}", serde_json::to_string(&r).unwrap());
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn launch_with_console_and_crash_hints() {
        use tauri::{Listener, Manager};
        let root = std::env::temp_dir().join("anilauncher-test");
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let file = auth::AccountsFile {
            accounts: vec![
                auth::Account { id: "m".into(), name: "Lic".into(), kind: "msa".into(), ..Default::default() },
                auth::Account { id: "o".into(), name: "Tester".into(), kind: "offline".into(), uuid: util::offline_uuid("Tester"), ..Default::default() },
            ],
            active: Some("o".into()),
        };
        auth::save(&st, &file).unwrap();
        let inst = instances::Instance {
            id: "live".into(), name: "Live".into(), mc_version: "1.21.11".into(), loader: "fabric".into(),
            memory_mb: Some(2048), width: Some(1000), height: Some(600), server: Some("play.example.net".into()),
            ..Default::default()
        };
        std::fs::create_dir_all(instances::game_dir(&st, "live")).unwrap();
        instances::store(&st, &inst).unwrap();
        let lines = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let l2 = lines.clone();
        app.listen("game-log", move |e| {
            let v: serde_json::Value = serde_json::from_str(e.payload()).unwrap();
            l2.fetch_add(v["lines"].as_array().map(|a| a.len()).unwrap_or(0), std::sync::atomic::Ordering::SeqCst);
        });
        let exited = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let x2 = exited.clone();
        app.listen("game-exited", move |e| {
            *x2.lock().unwrap() = Some(e.payload().to_string());
        });
        launch::launch_instance(app.handle().clone(), app.state(), "live".into(), None).await.unwrap();
        for _ in 0..90 {
            if exited.lock().unwrap().is_some() { break; }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let _ = launch::stop_instance(app.state(), "live".into());
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        println!("live lines received: {}", lines.load(std::sync::atomic::Ordering::SeqCst));
        println!("exit payload: {:?}", exited.lock().unwrap().as_deref().map(|s| s.chars().take(400).collect::<String>()));
        let log = std::fs::read_to_string(instances::dir(&st, "live").join("logs/latest-launch.log")).unwrap_or_default();
        println!("log lines: {}", log.lines().count());
    }

    #[tokio::test]
    #[ignore]
    async fn conflict_check_real_mods() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-pack-test");
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        for e in std::fs::read_dir(st.instances()).unwrap().flatten() {
            let id = e.file_name().to_string_lossy().to_string();
            let Ok(inst) = instances::load(&st, &id) else { continue };
            let n = std::fs::read_dir(instances::game_dir(&st, &id).join("mods")).map(|r| r.count()).unwrap_or(0);
            let p = conflicts::check_inner(&st, &id).await.unwrap();
            println!("== {} ({} {} {}, {} files): {} problems", inst.name, inst.loader, inst.mc_version, id, n, p.len());
            for x in p.iter().take(12) { println!("   {} {} {} -> {} [{}]", x.severity, x.kind, x.mod_name, x.other, x.detail); }
        }
    }

    #[tokio::test]
    async fn restore_point_roundtrip() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-snap-test");
        let _ = std::fs::remove_dir_all(&root);
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let inst = instances::Instance { id: "s1".into(), name: "Snap".into(), mc_version: "1.21.1".into(), loader: "fabric".into(), play_seconds: 10, ..Default::default() };
        let game = instances::game_dir(&st, "s1");
        std::fs::create_dir_all(game.join("mods")).unwrap();
        std::fs::create_dir_all(game.join("config/sub")).unwrap();
        std::fs::create_dir_all(game.join("saves/World")).unwrap();
        instances::store(&st, &inst).unwrap();
        std::fs::write(game.join("mods/a.jar"), b"A1").unwrap();
        std::fs::write(game.join("config/sub/c.toml"), b"x=1").unwrap();
        std::fs::write(game.join("saves/World/level.dat"), b"keep").unwrap();
        let snap = snapshots::create_snapshot(app.state(), "s1".into(), "before".into()).await.unwrap();
        std::fs::write(game.join("mods/a.jar"), b"A2").unwrap();
        std::fs::write(game.join("mods/b.jar"), b"B").unwrap();
        std::fs::write(game.join("config/sub/c.toml"), b"x=2").unwrap();
        let mut changed = instances::load(&st, "s1").unwrap();
        changed.mc_version = "1.21.4".into();
        changed.play_seconds = 99;
        instances::store(&st, &changed).unwrap();
        snapshots::restore_snapshot(app.state(), "s1".into(), snap.file.clone()).await.unwrap();
        assert_eq!(std::fs::read(game.join("mods/a.jar")).unwrap(), b"A1");
        assert!(!game.join("mods/b.jar").exists());
        assert_eq!(std::fs::read(game.join("config/sub/c.toml")).unwrap(), b"x=1");
        assert_eq!(std::fs::read(game.join("saves/World/level.dat")).unwrap(), b"keep");
        let after = instances::load(&st, "s1").unwrap();
        assert_eq!(after.mc_version, "1.21.1");
        assert_eq!(after.play_seconds, 99);
        assert_eq!(snapshots::list_snapshots(app.state(), "s1".into()).len(), 2);
    }

    #[tokio::test]
    #[ignore]
    async fn share_links_and_drops() {
        use tauri::Manager;
        let root = std::env::temp_dir().join("anilauncher-pack-test");
        let app = tauri::test::mock_app();
        app.manage(state::AppState::with_root(root.clone()));
        let st = app.state::<state::AppState>();
        let text = "Мой пак:\nhttps://modrinth.com/mod/sodium\nhttps://modrinth.com/mod/create\nhttps://www.curseforge.com/minecraft/mc-mods/jei\n- Iris Shaders\n2. Mod Menu\nnonexistentmodxyz12345";
        let r = friend::resolve_share(app.state(), text.into(), Some("fabric".into()), Some("1.21.1".into())).await.unwrap();
        for i in serde_json::to_value(&r).unwrap()["items"].as_array().unwrap() { println!("found: {} {} {}", i["title"], i["source"], i["available"]); }
        let jar = std::fs::read_dir(instances::game_dir(&st, "opt").join("mods")).unwrap().flatten().find(|e| e.file_name().to_string_lossy().starts_with("sodium")).unwrap().path();
        let d = friend::analyze_drop(app.state(), jar.to_string_lossy().into()).await.unwrap();
        println!("drop: {}", serde_json::to_string(&d).unwrap());
    }
}
