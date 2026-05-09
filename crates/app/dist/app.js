// Drop-zone + video player wired through Tauri's drag-drop event API.

const { event, core } = window.__TAURI__ ?? {};
const { listen } = event ?? {};
const { convertFileSrc } = core ?? {};

const dropZone = document.getElementById("drop-zone");
const errorMessage = document.getElementById("error-message");
const player = document.getElementById("player");
const video = document.getElementById("video");
const resetButton = document.getElementById("reset-button");

let errorTimer;

function showError(message) {
  errorMessage.textContent = message;
  errorMessage.hidden = false;
  clearTimeout(errorTimer);
  errorTimer = setTimeout(() => {
    errorMessage.hidden = true;
  }, 3000);
}

function isMp4(path) {
  return path.toLowerCase().endsWith(".mp4");
}

function loadVideo(path) {
  if (typeof convertFileSrc !== "function") {
    showError("Tauri asset bridge unavailable");
    return;
  }
  const url = convertFileSrc(path);
  video.src = url;
  player.hidden = false;
  dropZone.hidden = true;
  video.play().catch(() => {});
}

function reset() {
  video.pause();
  video.removeAttribute("src");
  video.load();
  player.hidden = true;
  dropZone.hidden = false;
}

resetButton.addEventListener("click", reset);

if (typeof listen === "function") {
  listen("tauri://drag-enter", () => {
    dropZone.classList.add("dragging");
  });
  listen("tauri://drag-leave", () => {
    dropZone.classList.remove("dragging");
  });
  listen("tauri://drag-drop", (e) => {
    dropZone.classList.remove("dragging");
    const paths = e.payload?.paths ?? [];
    const mp4 = paths.find(isMp4);
    if (mp4) {
      loadVideo(mp4);
    } else if (paths.length > 0) {
      showError("Only MP4 files are supported");
    }
  });
} else {
  showError("Tauri runtime not detected — open this through `cargo tauri dev`");
}
