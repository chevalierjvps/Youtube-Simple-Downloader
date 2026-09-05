use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize)]
struct ProgressPayload {
    kind: String, // "progress" | "postprocess"
    percent: Option<f64>,
    speed: Option<String>,
    eta: Option<String>,
    title: Option<String>,
    playlist_index: Option<u32>,
    playlist_count: Option<u32>,
}

#[derive(Clone, Serialize)]
struct FinishedPayload {
    success: bool,
    code: Option<i32>,
}

#[tauri::command]
fn check_dependencies() -> serde_json::Value {
    let ytdlp = Command::new("yt-dlp").arg("--version").output().is_ok();
    let ffmpeg = Command::new("ffmpeg").arg("-version").output().is_ok();
    serde_json::json!({ "ytdlp": ytdlp, "ffmpeg": ffmpeg })
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    log::info!("pick_folder invoked");
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder.map(|p| p.to_string()));
    });
    let result = rx.await.unwrap_or(None);
    log::info!("pick_folder result: {result:?}");
    result
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn start_download(
    app: AppHandle,
    url: String,
    media_type: String,
    audio_format: Option<String>,
    audio_quality: Option<String>,
    video_max_height: Option<String>,
    video_container: Option<String>,
    output_dir: String,
    playlist_mode: Option<bool>,
) {
    thread::spawn(move || {
        let mut args: Vec<String> = vec![
            "--newline".into(),
            "--ignore-errors".into(),
            "--no-overwrites".into(),
        ];

        match playlist_mode {
            Some(true) => args.push("--yes-playlist".into()),
            Some(false) => args.push("--no-playlist".into()),
            None => {}
        }

        let download_tmpl = "PROGRESS|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(info.title)s|%(info.playlist_index)s|%(info.playlist_count)s";
        let postprocess_tmpl = "POSTPROCESS|%(info.title)s";
        args.push("--progress-template".into());
        args.push(format!("download:{download_tmpl}"));
        args.push("--progress-template".into());
        args.push(format!("postprocess:{postprocess_tmpl}"));

        if media_type == "audio" {
            let fmt = audio_format.unwrap_or_else(|| "mp3".into());
            let quality = audio_quality.unwrap_or_else(|| "0".into());
            args.push("-x".into());
            args.push("--audio-format".into());
            args.push(fmt);
            args.push("--audio-quality".into());
            args.push(quality);
            args.push("--embed-thumbnail".into());
            args.push("--convert-thumbnails".into());
            args.push("jpg".into());
            args.push("--embed-metadata".into());
            args.push("--add-metadata".into());
        } else {
            let format_sel = match video_max_height.as_deref() {
                Some(h) if h != "best" => {
                    format!("bestvideo[height<={h}]+bestaudio/best[height<={h}]")
                }
                _ => "bestvideo+bestaudio/best".to_string(),
            };
            let container = video_container.unwrap_or_else(|| "mp4".into());
            args.push("-f".into());
            args.push(format_sel);
            args.push("--merge-output-format".into());
            args.push(container);
            args.push("--embed-thumbnail".into());
            args.push("--embed-metadata".into());
            args.push("--add-metadata".into());
        }

        let outtmpl = if matches!(playlist_mode, Some(true)) {
            format!("{output_dir}/%(playlist_index)03d - %(title)s.%(ext)s")
        } else {
            format!("{output_dir}/%(title)s.%(ext)s")
        };
        args.push("-o".into());
        args.push(outtmpl);
        args.push(url);

        let mut child = match Command::new("yt-dlp")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("ytdl://log", format!("Failed to start yt-dlp: {e}"));
                let _ = app.emit(
                    "ytdl://finished",
                    FinishedPayload {
                        success: false,
                        code: None,
                    },
                );
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let app_stdout = app.clone();
        let stdout_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                handle_line(&app_stdout, &line);
            }
        });

        let app_stderr = app.clone();
        let stderr_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if !line.trim().is_empty() {
                    let _ = app_stderr.emit("ytdl://log", line);
                }
            }
        });

        let status = child.wait();
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        let (success, code) = match status {
            Ok(s) => (s.success(), s.code()),
            Err(_) => (false, None),
        };
        let _ = app.emit("ytdl://finished", FinishedPayload { success, code });
    });
}

fn handle_line(app: &AppHandle, line: &str) {
    if let Some(rest) = line.strip_prefix("PROGRESS|") {
        let parts: Vec<&str> = rest.split('|').collect();
        if parts.len() >= 6 {
            let percent = parts[0].trim().trim_end_matches('%').parse::<f64>().ok();
            let speed = Some(parts[1].trim().to_string());
            let eta = Some(parts[2].trim().to_string());
            let title = Some(parts[3].trim().to_string());
            let playlist_index = parts[4].trim().parse::<u32>().ok();
            let playlist_count = parts[5].trim().parse::<u32>().ok();
            let _ = app.emit(
                "ytdl://progress",
                ProgressPayload {
                    kind: "progress".into(),
                    percent,
                    speed,
                    eta,
                    title,
                    playlist_index,
                    playlist_count,
                },
            );
        }
    } else if let Some(rest) = line.strip_prefix("POSTPROCESS|") {
        let _ = app.emit(
            "ytdl://progress",
            ProgressPayload {
                kind: "postprocess".into(),
                percent: None,
                speed: None,
                eta: None,
                title: Some(rest.trim().to_string()),
                playlist_index: None,
                playlist_count: None,
            },
        );
    } else if !line.trim().is_empty() {
        let _ = app.emit("ytdl://log", line.to_string());
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Work around a WebKitGTK "Failed to create GBM buffer" gray-screen bug that
    // shows up on some Mesa/driver combinations, especially in AppImage builds
    // that bundle their own WebKitGTK. Must be set before the webview inits.
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            pick_folder,
            start_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
