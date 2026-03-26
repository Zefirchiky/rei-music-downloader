use std::{
    fmt::Display, io::{BufRead, BufReader}, process::{ChildStdout, Command, Stdio}
};

use filess::Ogg;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{Artist, Track, YtDlpCli};

#[derive(Debug)]
pub struct SearchResult {
    pub uploader: Artist,
    pub title: Track,
    pub url: String,
    pub duration: usize,
}

impl Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration_min = self.duration / 60;
        let duration_sec = self.duration % 60;
        
        let s = if self.title.starts_with(&format!("{} - ", &self.uploader)) {
            format!("{} ({:0>2}:{:0>2}) [{}]", &self.title, duration_min, duration_sec, &self.url)
        } else {
            format!("{} - {} ({:0>2}:{:0>2}) [{}]",
                &self.uploader,
                &self.title,
                duration_min,
                duration_sec,
                &self.url,
            )
        };
        f.write_str(&s)
    }
}

#[derive(Debug)]
pub struct TrackInfo {
    pub artist: Option<Artist>,
    pub uploader: Artist,      // Fallback for "author"
    pub track: Option<Track>, // Specific song name if available
    pub title: Track,
    pub file: Ogg,
    pub url: String,
}

pub struct YtDlp {
    pub command: Command,
}

impl YtDlp {
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let mut cmd = YtDlpCli::setup_search_command(query, limit);
        
        let output = cmd
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start search")
            .wait_with_output()
            .expect("Failed to read search output");

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split("|||").collect();
                if parts.len() == 4 {
                    Some(SearchResult {
                        uploader: parts[0].into(),
                        title: parts[1].into(),
                        url: parts[2].to_string(),
                        duration: parts[3].parse::<f32>().unwrap() as usize,
                    })
                } else {
                    None
                }
            })
            .collect()
        }
    
    pub fn download(&mut self, query: &str) -> TrackInfo {
        let mut child = self
            .command
            .arg(query)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start yt-dlp");

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let reader = BufReader::new(stdout);

        let pb = Self::setup_progress_bar();
        let collected_lines = Self::parse_stdout(reader, &pb);

        let status = child.wait().expect("Process failed to finish");
        pb.finish_with_message("Download Complete");

        if !status.success() {
            panic!("yt-dlp exited with an error");
        }

        if collected_lines.len() < 6 {
            panic!("Failed to capture all metadata. Got: {:?}", collected_lines);
        }

        let clean = |s: &str| if s == "NA" { None } else { Some(s.to_string()) };

        TrackInfo {
            artist: clean(&collected_lines[0]).map(|s| s.into()),
            uploader: collected_lines[1].clone().into(),
            track: clean(&collected_lines[2]).map(|s| s.into()),
            title: collected_lines[3].clone().into(),
            url: collected_lines[4].clone(),
            file: Ogg::new(collected_lines[5].clone()),
        }
    }

    fn setup_progress_bar() -> ProgressBar {
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] 『{bar}』 {pos}% ({eta})",
                )
                .unwrap()
                .progress_chars(crate::PROGRESS_CHARS),
        );
        pb
    }

    fn parse_stdout(reader: BufReader<ChildStdout>, pb: &ProgressBar) -> Vec<String> {
        // Regex to find "[download]  10.5%" patterns
        let progress_re = regex::Regex::new(r"\[download\]\s+(\d+\.?\d*)%").unwrap();
        let mut collected_lines = Vec::new();

        for l in reader.lines().map_while(Result::ok) {
            if let Some(caps) = progress_re.captures(&l) {
                if let Ok(p) = caps[1].parse::<f32>() {
                    pb.set_position(p as u64);
                }
            } else {
                if !l.trim().is_empty() {
                    collected_lines.push(l.to_string());
                }
            }
        }

        collected_lines
    }
}

impl Default for YtDlp {
    fn default() -> Self {
        Self {
            command: YtDlpCli::setup_download_command(),
        }
    }
}
