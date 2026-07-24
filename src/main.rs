use std::{
    fmt::Display,
    io::{self, Write},
};

use filess::traits::FileTrait;
use inquire::{MultiSelect, Select, Text, validator::MinLengthValidator};

pub mod components;
mod lyrics;
mod parser;
pub mod song;
mod yt_dlp;
mod yt_dlp_cli;

pub use lyrics::*;
pub use parser::*;
use reqwest::Url;
pub use song::*;
pub use yt_dlp::*;
pub use yt_dlp_cli::*;

use crate::components::{Artist, ArtistOrRemixArtist, RemixArtist, Track};

pub const VERSION: &str = "0.2.0";

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

fn get_with_select_multiple<T: Display + Clone>(
    select: &str,
    options: Vec<T>,
    hint: &str,
) -> Vec<T> {
    if options.len() > 1 {
        MultiSelect::new(
            &format!("Select {select} (or best matches to edit):"),
            options,
        )
        .with_help_message(&["Press <Space> to toggle, <Enter> to submit", hint].join("\n"))
        .with_validator(
            MinLengthValidator::new(1).with_message("You must select at least one option!"),
        )
        .prompt()
        .unwrap()
    } else {
        vec![options.first().unwrap().clone()]
    }
}

fn parse_url_from_prompt(prompt: String, dlp: &YtDlp) -> (ParsedTrack, String) {
    let mut parsed_candidates =
        Parser::parse_title(prompt.clone()).expect("Prompt mustn't be empty");

    let url = if prompt.starts_with("http://") || prompt.starts_with("https://") {
        prompt
    } else {
        let searches = dlp.search(&prompt, 20);
        let choice = Select::new("Select from available video:", searches)
            .prompt()
            .unwrap();

        let p2 = Parser::parse_title(choice.title.as_ref().to_string())
            .expect("Downloaded title can't be empty"); // FIXME: It probably can be empty
        parsed_candidates = parsed_candidates + p2;
        parsed_candidates.artists.push(choice.uploader);
        choice.url
    };

    (parsed_candidates, url)
}

fn choose_lyrics(options: Vec<LyricsResponse>, url: &Url) -> Option<LyricsResponse> {
    inquire::Select::new(&format!("Got {} lyrics, choose:", options.len()), options)
        .with_help_message(&format!("Url: {url}"))
        .prompt_skippable()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let prompt_string = Text::new("Enter the song name or URL:")
        .with_placeholder("e.g. Never Gonna Give You Up")
        .prompt()
        .unwrap()
        .replace("feat.", "ft.");

    let mut dlp = YtDlp::default();
    let (mut parsed_candidates, url) = parse_url_from_prompt(prompt_string.clone(), &dlp);

    let output = dlp.download(&url);
    if let Some(artist) = output.artist.clone() {
        parsed_candidates.artists.extend(artist.as_ref().split(", ").map(Artist::from).collect::<Vec<_>>());
    }
    parsed_candidates.artists.extend(Parser::extract_artists(&output.uploader));
    if let Some(track) = output.track.clone() {
        parsed_candidates = parsed_candidates + Parser::parse_title(track.as_ref().to_string()).unwrap()
    }

    if prompt_string.starts_with("http://") || prompt_string.starts_with("https://") {
        let title = output.title;
        parsed_candidates = parsed_candidates
            + Parser::parse_title(title.as_ref().to_string())
                .expect("Downloaded title mustn't be empty"); // FIXME: Same issue
    }

    parsed_candidates.dedup();

    let mut artist_hints = "Found artist(s): [".to_string();
    artist_hints.push_str(
        &parsed_candidates
            .artists
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );
    artist_hints.push_str("]");

    let mut track_hints = "Found track(s): [".to_string();
    track_hints.push_str(
        &parsed_candidates
            .tracks
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    );
    track_hints.push_str("]");

    let hints = vec![artist_hints, track_hints].join(",\n");
    let artists_remix_artists_candidates: Vec<ArtistOrRemixArtist> = parsed_candidates
        .artists
        .iter()
        .map(|a| ArtistOrRemixArtist::Artist(a.clone()))
        .chain(
            parsed_candidates
                .remix_artists
                .iter()
                .map(|ra| ArtistOrRemixArtist::RemixArtist(ra.clone())),
        )
        .collect();
    let artists_remix_artists =
        get_with_select_multiple("Artist", artists_remix_artists_candidates, &hints);
    let track = get_with_select("Name", parsed_candidates.tracks, &hints);

    let mut lyrics = None;

    let full_artist_string = artists_remix_artists
        .iter()
        .map(|a| a.as_ref().to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let mut searches = vec![(Some(full_artist_string.clone()), track.clone())]; // TODO: Separator customization

    let just_artists: Vec<Artist> = artists_remix_artists
        .iter()
        .filter_map(|a| match a {
            ArtistOrRemixArtist::Artist(artist) => Some(artist.clone()),
            ArtistOrRemixArtist::RemixArtist(_) => None,
        })
        .collect();

    searches.push((
        Some(
            just_artists
                .iter()
                .map(|a| a.as_ref().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        track.clone(),
    ));
    searches.push((None, track.clone()));

    let lyrics_struct = Lyrics::default();
    let mut is_instrumental = false;
    for (a, t) in searches {
        let (url, results) = lyrics_struct.search_with_track_artist(a.as_ref(), &t).await;
        if !results.is_empty() {
            let choice = choose_lyrics(results, &url);
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
            &full_artist_string, &track
        );
        let search_string = Text::new("Search with name:")
            .prompt_skippable()
            .unwrap()
            .map(|s| Track::new(&s));
        if let Some(search_string) = search_string
            && !search_string.is_empty()
        {
            let (url, results) = lyrics_struct.search(&search_string).await;

            if !results.is_empty() {
                let choice = choose_lyrics(results, &url);
                if let Some(choice) = choice {
                    if !choice.instrumental {
                        lyrics = choice.synced_lyrics.or(choice.plain_lyrics);
                    }
                };
            }
        }
    }

    let mut remixes_string = "".to_string();
    if !parsed_candidates.remixes.is_empty() {
        remixes_string = " [".to_string()
            + &parsed_candidates
                .remixes
                .iter()
                .map(|r| r.to_uppercase())
                .collect::<Vec<String>>()
                .join(" & ")
            + "]";
    }
    let file = output
        .file
        .copy(&format!(
            "{SAVE_PATH}/{full_artist_string} - {track}{remixes_string}.opus"
        )) // TODO: Format setting
        .unwrap();
    output.file.remove().unwrap();

    let mut artists = vec![];
    let mut remix_artists = vec![];

    for ara in artists_remix_artists {
        match ara {
            ArtistOrRemixArtist::Artist(a) => artists.push(a.clone()),
            ArtistOrRemixArtist::RemixArtist(ra) => remix_artists.push(ra.clone()),
        }
    }

    let song = Song {
        file,
        artists,
        remix_artists,
        track,
        remixes: parsed_candidates.remixes,
        lyrics,
        url: output.url,
    };
    song.fix_metadata();

    if song.file.exists() {
        log_step(&format!(
            "Successfully saved: {}",
            song.file.path.to_str().unwrap()
        ));
        // sleep(Duration::from_millis(100)); // YtDlp might not remove all temp files in time
    } else {
        panic!("Something gone wrong, file wasn't saved, try again");
    }
}
