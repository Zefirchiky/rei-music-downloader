use std::{
    fmt::Display,
    fs::remove_dir,
    io::{self, Write},
    thread::sleep,
    time::Duration,
};

use filess::traits::FileTrait;
use inquire::{Select, Text};

mod artist;
mod lyrics;
mod song;
mod track;
mod yt_dlp;
mod yt_dlp_cli;

pub use artist::*;
pub use lyrics::*;
pub use song::*;
pub use track::*;
pub use yt_dlp::*;
pub use yt_dlp_cli::*;

pub const PROGRESS_CHARS: &str = "█▉▊▋▌▍▎▏ ";
pub const ARTIST_DIVIDER_CHARS: [char; 4] = ['&', '/', '|', ','];

pub const SAVE_PATH: &str = "/home/rei/Music";

pub fn log_step(message: &str) {
    // \r moves cursor to start, \x1b[K clears the rest of the line
    print!("\r\x1b[K[ STATUS ] {}", message);
    io::stdout().flush().unwrap();
}

fn get_with_select<T: Display + AsRef<str> + From<String> + Clone>(
    select: &str,
    options: Vec<T>,
    hint: &str,
) -> T {
    let choice = if options.len() > 1 {
        Select::new(
            &format!("Select {select} (or best match to edit):"),
            options,
        )
        .with_help_message(hint)
        .prompt()
        .unwrap()
    } else {
        options.first().unwrap().clone()
    };

    Text::new("Confirm/Edit:")
        .with_initial_value(choice.as_ref()) // This pre-fills the input with their selection
        .prompt()
        .unwrap()
        .into()
}

#[tokio::main]
async fn main() {
    let prompt_string = Text::new("Enter the song name or URL:")
        .with_placeholder("e.g. Never Gonna Give You Up")
        .prompt()
        .unwrap()
        .replace("feat.", "ft.");

    let mut artist_candidates = vec![];
    let mut track_candidates = vec![];

    let mut dlp = YtDlp::default();
    let url = if prompt_string.starts_with("http://") || prompt_string.starts_with("https://") {
        prompt_string
    } else {
        let t1 = Track::new(&prompt_string);
        if let Some((a, t)) = t1.try_extract() {
            artist_candidates.push(a);
            track_candidates.push(t);
        } else {
            track_candidates.push(t1);
        }

        let searches = dlp.search(&prompt_string, 20);
        let choice = Select::new("Select from available video:", searches)
            .prompt()
            .unwrap();

        let mut title = choice.title.clone();
        while let Some((a, t)) = title.try_extract() {
            if !artist_candidates.contains(&a) {
                artist_candidates.push(a);
            }
            if !track_candidates.contains(&t) {
                track_candidates.push(t.clone());
            }
            title = t;
        }
        if !track_candidates.contains(&title) {
            track_candidates.push(title);
        }
        if !artist_candidates.contains(&choice.uploader) {
            artist_candidates.push(choice.uploader);
        }
        choice.url
    };

    let output = dlp.download(&url);

    let mut artist_hints = "Found artist(s): [".to_string();
    artist_hints.push_str(
        &artist_candidates
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );
    artist_hints.push_str("]");

    let mut track_hints = "Found track(s): [".to_string();
    track_hints.push_str(
        &track_candidates
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );
    track_hints.push_str("]");

    let hints = vec![artist_hints, track_hints].join(", ");
    let artist = get_with_select("Artist", artist_candidates, &hints);
    let track = get_with_select("Name", track_candidates, &hints);

    let mut lyrics = None;
    let mut searches = vec![(Some(&artist), track.clone())];

    if let Some(at) = track.find('(') {
        let track: Track = track[..at].into();
        searches.push((Some(&artist), track.clone()));
        searches.push((None, track));
    }

    let mut is_instrumental = false;
    for (a, t) in searches {
        let (url, results) = Lyrics::default().search(a, &t).await;
        if !results.is_empty() {
            let choice = Select::new(&format!("Got {} lyrics, choose:", results.len()), results)
                .with_help_message(&format!("Url: {url}"))
                .prompt_skippable()
                .unwrap();
            if let Some(choice) = choice {
                if choice.instrumental {
                    is_instrumental = true;
                    break;
                }
                lyrics = choice.synced_lyrics.or(choice.plain_lyrics);
                break; // Exit loop once lyrics are found
            }
        }
    }

    if lyrics.is_none() && !is_instrumental {
        println!(
            "Not found on lrclib. Search: https://lrclib.net/search/{} - {}",
            artist, track
        );
        let search_string = Text::new("Search with name:")
            .prompt_skippable()
            .unwrap()
            .map(|s| Track::new(&s));
        if let Some(search_string) = search_string
            && !search_string.is_empty()
        {
            let mut search_artist = None;
            let mut search_track = search_string.clone();
            if let Some((artist, name)) = search_track.try_extract() {
                search_artist = Some(artist);
                search_track = name
            }
            let (url, results) = Lyrics::default()
                .search(search_artist.as_ref(), &search_track)
                .await;

            if !results.is_empty() {
                let choice = inquire::Select::new(
                    &format!("Got {} lyrics, choose:", results.len()),
                    results,
                )
                .with_help_message(&format!("Url: {url}"))
                .prompt_skippable()
                .unwrap();
                if let Some(choice) = choice {
                    lyrics = choice.synced_lyrics.or(choice.plain_lyrics);
                }
            }
        }
    }

    let file = output
        .file
        .rename(&format!("{SAVE_PATH}/{artist} - {track}.opus"))
        .unwrap();
    let song = Song {
        file,
        artist,
        track,
        lyrics,
        url: output.url,
    };
    song.fix_metadata();
    log_step(&format!(
        "Successfully saved: {}",
        song.file.path.to_str().unwrap()
    ));
    sleep(Duration::from_millis(100)); // YtDlp might not remove all temp files in time
    remove_dir(format!("{SAVE_PATH}/tmp")).unwrap();
}
