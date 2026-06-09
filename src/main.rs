use std::process::Command;
use std::thread;
use std::io::Cursor;
use serde::{Serialize, Deserialize};
use slint::ComponentHandle;
use rodio::{Decoder, OutputStream};

slint::include_modules!();

const CLICK_SOUND_BYTES: &[u8] = include_bytes!("sound.wav");

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LauncherConfig {
    proton_path: String,
    prefix_path: String,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            proton_path: "~/proton".to_string(),
            prefix_path: "~/.pfx".to_string(),
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = MainWindow::new()?;

    let initial_config: LauncherConfig = confy::load("humra-dumra", "config").unwrap_or_default();
    main_window.set_proton_path(initial_config.proton_path.as_str().into());
    main_window.set_prefix_path(initial_config.prefix_path.as_str().into());

    let (_stream, stream_handle) = OutputStream::try_default()
    .expect("Failed to open default audio output device");

    let window_weak = main_window.as_weak();

    main_window.on_browse_exe_clicked({
        let window_weak = window_weak.clone();
        move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Windows Executables", &["exe"])
                .pick_file()
                {
                    if let Some(window) = window_weak.upgrade() {
                        let path_str = path.to_string_lossy().into_owned();
                        window.set_game_path(path_str.into());
                    }
                }
        }
    });

    main_window.on_browse_proton_clicked({
        let window_weak = window_weak.clone();
        move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select Proton Folder")
                .pick_folder()
                {
                    if path.join("proton").exists() || path.join("toolmanifest.vdf").exists() {
                        let path_str = path.to_string_lossy().into_owned();

                        if let Some(window) = window_weak.upgrade() {
                            window.set_proton_path(path_str.as_str().into());
                        }

                        if let Ok(mut config) = confy::load::<LauncherConfig>("humra-dumra", "config") {
                            config.proton_path = path_str;
                            if let Err(e) = confy::store("humra-dumra", "config", &config) {
                                eprintln!("ERROR: Failed to save proton_path to configuration file: {}", e);
                            } else {
                                println!("NOTICE: The proton_path has been successfully written to disk.");
                            }
                        }
                    } else {
                        eprintln!("WARNING: Selected folder does not appear to be a valid Proton version.");
                    }
                }
        }
    });


    main_window.on_browse_prefix_clicked({
        let window_weak = window_weak.clone();
        move || {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Select Prefix Folder")
                .pick_folder()
                {
                    let path_str = path.to_string_lossy().into_owned();
                    if let Some(window) = window_weak.upgrade() {
                        window.set_prefix_path(path_str.as_str().into());
                    }

                    if let Ok(mut config) = confy::load::<LauncherConfig>("humra-dumra", "config") {
                        config.prefix_path = path_str;
                        if let Err(e) = confy::store("humra-dumra", "config", &config) {
                            eprintln!("ERROR: Failed to save prefix_path to configuration file: {}", e);
                        } else {
                            println!("NOTICE: The prefix_path has been successfully written to disk.");
                        }
                    }
                }
        }
    });


    let audio_handle = stream_handle.clone();
    main_window.on_enter_dashboard_clicked(move |game_path| {
        println!("Launch button triggered. Reading configurations from file...");

        let cursor = Cursor::new(CLICK_SOUND_BYTES);

        if let Ok(source) = Decoder::new_wav(cursor) {
            let _ = audio_handle.play_raw(rodio::Source::convert_samples(source));
        } else {
            eprintln!("ERROR: Failed to decode embedded WAV audio asset memory block.");
        }

        let config: LauncherConfig = confy::load("humra-dumra", "config").unwrap_or_default();
        let target_exe = game_path.to_string();
        let proton_path = config.proton_path;
        let prefix_path = config.prefix_path;

        thread::spawn(move || {
            if proton_path.contains("~/proton") {
                eprintln!("WARNING: You are using a placeholder config! Please edit the config file: ~/.config/humra-dumra/config.toml");
            }

            let mut cmd = Command::new("umu-run");
            cmd.arg(&target_exe)
            .env("PROTONPATH", &proton_path);

            if !prefix_path.is_empty() {
                cmd.env("WINEPREFIX", &prefix_path);
            }

            println!("Launching: umu-run {}", target_exe);
            println!("Loaded environment variable for PROTONPATH: {}", proton_path);
            println!("Loaded environment variable for WINEPREFIX: {}", prefix_path);

            match cmd.output() {
                Ok(out) => {
                    if out.status.success() {
                        println!("The executable has been gracefully closed...");
                    } else {
                        eprintln!(
                            "ERROR: umu-run exited with error: {}",
                            String::from_utf8_lossy(&out.stderr)
                        );
                    }
                }
                Err(e) => {
                    eprintln!("ERROR: umu-run binary not found...");
                    eprintln!("Detail: {}", e);
                }
            }
        });
    });

    main_window.run()
}
