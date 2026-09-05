const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const el = (id) => document.getElementById(id);

const urlInput = el("url");
const mediaTypeGroup = el("media-type");
const audioOptions = el("audio-options");
const videoOptions = el("video-options");
const audioFormat = el("audio-format");
const audioQuality = el("audio-quality");
const videoQuality = el("video-quality");
const videoContainer = el("video-container");
const outputDirInput = el("output-dir");
const browseBtn = el("browse-btn");
const playlistField = el("playlist-field");
const playlistCheckbox = el("playlist-checkbox");
const downloadBtn = el("download-btn");
const progressArea = el("progress-area");
const progressTitle = el("progress-title");
const progressBarFill = el("progress-bar-fill");
const progressSpeed = el("progress-speed");
const progressEta = el("progress-eta");
const statusMsg = el("status-msg");
const depsWarning = el("deps-warning");
const themeToggle = el("theme-toggle");
const themeIcon = el("theme-icon");
const logDetails = el("log-details");
const logBox = el("log-box");

let mediaType = "audio";
let isDownloading = false;

// ---------- Theme ----------
function applyTheme(theme) {
  if (theme === "light") {
    document.documentElement.setAttribute("data-theme", "light");
    themeIcon.innerHTML = "&#9788;"; // sun
  } else {
    document.documentElement.setAttribute("data-theme", "dark");
    themeIcon.innerHTML = "&#9789;"; // moon
  }
  localStorage.setItem("ytdl-theme", theme);
}

(function initTheme() {
  const saved = localStorage.getItem("ytdl-theme");
  const prefersLight = window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches;
  applyTheme(saved || (prefersLight ? "light" : "dark"));
})();

themeToggle.addEventListener("click", () => {
  const current = document.documentElement.getAttribute("data-theme");
  applyTheme(current === "light" ? "dark" : "light");
});

// ---------- Media type toggle ----------
mediaTypeGroup.querySelectorAll(".seg-btn").forEach((btn) => {
  btn.addEventListener("click", () => {
    mediaTypeGroup.querySelectorAll(".seg-btn").forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    mediaType = btn.dataset.value;
    audioOptions.hidden = mediaType !== "audio";
    videoOptions.hidden = mediaType !== "video";
  });
});

// ---------- Playlist detection ----------
function detectPlaylist(url) {
  let parsed;
  try {
    parsed = new URL(url);
  } catch {
    return { hasList: false, hasVideo: false };
  }
  const hasList = parsed.searchParams.has("list");
  const hasVideo = parsed.searchParams.has("v") || parsed.hostname.includes("youtu.be");
  return { hasList, hasVideo };
}

urlInput.addEventListener("input", () => {
  const { hasList, hasVideo } = detectPlaylist(urlInput.value.trim());
  playlistField.hidden = !(hasList && hasVideo);
});

// ---------- Folder picker ----------
browseBtn.addEventListener("click", async () => {
  try {
    const dir = await invoke("pick_folder");
    if (dir) {
      outputDirInput.value = dir;
    }
  } catch (e) {
    console.error(e);
    statusMsg.hidden = false;
    statusMsg.className = "status-msg error";
    statusMsg.textContent = `Could not open folder picker: ${e}`;
  }
});

// ---------- Dependency check ----------
async function checkDeps() {
  try {
    const status = await invoke("check_dependencies");
    if (!status.ytdlp || !status.ffmpeg) {
      const missing = [];
      if (!status.ytdlp) missing.push("yt-dlp");
      if (!status.ffmpeg) missing.push("ffmpeg");
      depsWarning.textContent = `Missing: ${missing.join(", ")}. Install ${missing.join(
        " and "
      )} and make sure they are on your PATH, then restart the app.`;
      depsWarning.hidden = false;
    }
  } catch (e) {
    console.error(e);
  }
}
checkDeps();

// ---------- Log ----------
function appendLog(line) {
  logDetails.hidden = false;
  logBox.textContent += line + "\n";
  logBox.scrollTop = logBox.scrollHeight;
}

// ---------- Download ----------
function setDownloading(active) {
  isDownloading = active;
  downloadBtn.disabled = active;
  downloadBtn.textContent = active ? "Downloading..." : "Download";
}

function resetProgress() {
  progressArea.hidden = false;
  progressTitle.textContent = "Starting...";
  progressBarFill.style.width = "0%";
  progressSpeed.textContent = "";
  progressEta.textContent = "";
  statusMsg.hidden = true;
  logBox.textContent = "";
  logDetails.hidden = true;
}

downloadBtn.addEventListener("click", async () => {
  if (isDownloading) return;

  const url = urlInput.value.trim();
  const outputDir = outputDirInput.value.trim();

  if (!url) {
    statusMsg.hidden = false;
    statusMsg.className = "status-msg error";
    statusMsg.textContent = "Please paste a URL first.";
    return;
  }
  if (!outputDir) {
    statusMsg.hidden = false;
    statusMsg.className = "status-msg error";
    statusMsg.textContent = "Please choose an output folder first.";
    return;
  }

  const { hasList, hasVideo } = detectPlaylist(url);
  let playlistMode = null;
  if (hasList && hasVideo) {
    playlistMode = playlistCheckbox.checked;
  } else if (hasList) {
    playlistMode = true;
  } else {
    playlistMode = false;
  }

  resetProgress();
  setDownloading(true);

  const payload = {
    url,
    mediaType,
    outputDir,
    playlistMode,
    audioFormat: mediaType === "audio" ? audioFormat.value : null,
    audioQuality: mediaType === "audio" ? audioQuality.value : null,
    videoMaxHeight: mediaType === "video" ? videoQuality.value : null,
    videoContainer: mediaType === "video" ? videoContainer.value : null,
  };

  try {
    await invoke("start_download", payload);
  } catch (e) {
    setDownloading(false);
    statusMsg.hidden = false;
    statusMsg.className = "status-msg error";
    statusMsg.textContent = `Failed to start: ${e}`;
  }
});

// ---------- Events from backend ----------
listen("ytdl://progress", (event) => {
  const p = event.payload;
  if (p.kind === "postprocess") {
    progressTitle.textContent = `Processing: ${p.title || ""}`;
    return;
  }
  let label = p.title || "Downloading...";
  if (p.playlist_index && p.playlist_count) {
    label = `[${p.playlist_index}/${p.playlist_count}] ${label}`;
  }
  progressTitle.textContent = label;
  if (typeof p.percent === "number" && !Number.isNaN(p.percent)) {
    progressBarFill.style.width = `${Math.min(100, Math.max(0, p.percent))}%`;
  }
  progressSpeed.textContent = p.speed || "";
  progressEta.textContent = p.eta ? `ETA ${p.eta}` : "";
});

listen("ytdl://log", (event) => {
  appendLog(event.payload);
});

listen("ytdl://finished", (event) => {
  setDownloading(false);
  const { success } = event.payload;
  statusMsg.hidden = false;
  if (success) {
    statusMsg.className = "status-msg success";
    statusMsg.textContent = `Done! Files saved to: ${outputDirInput.value}`;
    progressBarFill.style.width = "100%";
  } else {
    statusMsg.className = "status-msg error";
    statusMsg.textContent = "Finished with errors. Check details below.";
  }
});
