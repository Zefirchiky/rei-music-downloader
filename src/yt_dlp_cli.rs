use std::process::Command;

use crate::SAVE_PATH;

pub struct YtDlpCli {}

impl YtDlpCli {
    pub fn setup_search_command(query: &str, count: usize) -> Command {
        let mut command = Command::new("yt-dlp");
        let search_query = format!("ytsearch{count}:{query}");
        
        command
            .arg(search_query)
            .arg("--flat-playlist") // Fast: don't dive into video data
            .args(["--print", "%(uploader)s|||%(title)s|||%(webpage_url)s|||%(duration)s"]);
            
        command
    }
        
    pub fn setup_download_command() -> Command {
        let mut command = Command::new("yt-dlp");
        Self::setup_env(&mut command);
        Self::setup_print(&mut command);
        Self::setup_progress(&mut command);
        Self::setup_audio(&mut command);
        Self::setup_metadata(&mut command);
        Self::setup_cookie(&mut command);
        Self::setup_output(&mut command);
        command
    }

    fn setup_env(command: &mut Command) {
        command
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUNBUFFERED", "1");
    }

    fn setup_print(command: &mut Command) {
        let to_print = [
            "%(artist)s",
            "%(uploader)s",
            "%(track)s",
            "%(title)s",
            "%(webpage_url)s",
            "after_move:filepath",
        ];
        for tp in to_print {
            command.args(["--print", tp]);
        }
    }

    fn setup_progress(command: &mut Command) {
        command.arg("--newline").arg("--progress");
    }

    fn setup_audio(command: &mut Command) {
        command
            .arg("-x")
            .args(["--audio-format", "opus"])
            .args(["--audio-quality", "0"]);
    }

    fn setup_metadata(command: &mut Command) {
        command
            .arg("--force-overwrites") // Fixes random issues
            .arg("--no-part") // Fixes random issues
            .arg("--embed-thumbnail")
            .arg("--add-metadata");
    }

    fn setup_cookie(command: &mut Command) {
        command.args(["--cookies-from-browser", "firefox:/home/rei/.config/zen"]);
    }

    fn setup_output(command: &mut Command) {
        command.args(["-o", &format!("{SAVE_PATH}/tmp/%(title)s TEMP.%(ext)s")]);
    }
}
