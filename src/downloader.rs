use std::fmt::Display;

use filess::traits::FileTrait;
use inquire::{MultiSelect, Select, Text, validator::MinLengthValidator};
use reqwest::Url;

use crate::{
    YtDlp, components::{Artist, ArtistOrRemixArtist, Title}, log_step, lyrics::{Lyrics, LyricsResponse}, parser::{ParsedTrack, Parser}, song::Song,
};

pub struct Downloader {}

impl Downloader {
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

            let p2 = Parser::parse_title(choice.title.0.clone())
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

    pub fn prompt_input(message: &str, placeholder: &str) -> String {
        Text::new(message)
            .with_placeholder(placeholder)
            .prompt()
            .unwrap()
    }

    pub async fn download(&self, prompt: String) {
        let mut dlp = YtDlp::default();
        let (mut parsed_candidates, url) = Self::parse_url_from_prompt(prompt.clone(), &dlp);

        let output = dlp.download(&url);
        if let Some(artist) = output.artist.clone() {
            parsed_candidates.artists.extend(
                artist
                    .as_ref()
                    .split(", ")
                    .map(Artist::from)
                    .collect::<Vec<_>>(),
            );
        }
        parsed_candidates
            .artists
            .extend(Parser::extract_artists(&output.uploader));
        if let Some(track) = output.track.clone() {
            parsed_candidates =
                parsed_candidates + Parser::parse_title(track.0.clone()).unwrap()
        }

        if prompt.starts_with("http://") || prompt.starts_with("https://") {
            let title = output.title;
            parsed_candidates = parsed_candidates
                + Parser::parse_title(title.0.clone())
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
        parsed_candidates.dedup();
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
            Self::get_with_select_multiple("Artist", artists_remix_artists_candidates, &hints);
        let track = Self::get_with_select("Name", parsed_candidates.tracks, &hints);
        if !parsed_candidates.remixes.is_empty() {
            parsed_candidates.remixes = Self::get_with_select_multiple("Remixes", parsed_candidates.remixes.clone(), &hints);
        }

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
                    .map(|a| a.0.clone())
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
                let choice = Self::choose_lyrics(results, &url);
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
                .map(|s| Title::new(&s));
            if let Some(search_string) = search_string
                && !search_string.is_empty()
            {
                let (url, results) = lyrics_struct.search(&search_string).await;

                if !results.is_empty() {
                    let choice = Self::choose_lyrics(results, &url);
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

        let new_file = filess::Ogg::new(format!(
            "{}/{full_artist_string} - {track}{remixes_string}.opus",
            dirs::audio_dir().unwrap().into_string().unwrap()
        ));
        if new_file.exists() {
            new_file.trash().unwrap();
        }
        let file = output
            .file
            .copy(&new_file) // TODO: Format setting
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
            title: track,
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
            song.file.open().unwrap();
            // sleep(Duration::from_millis(100)); // YtDlp might not remove all temp files in time
        } else {
            panic!("Something gone wrong, file wasn't saved, try again");
        }
    }
}
