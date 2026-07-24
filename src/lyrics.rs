use std::fmt::Display;

use reqwest::Client;
use serde::Deserialize;

use crate::components::Track;

#[derive(Default)]
pub struct Lyrics {
    client: Client,
}

impl Lyrics {
    pub async fn search_with_query(
        &self,
        query: &[(&str, &str)],
    ) -> (reqwest::Url, Vec<LyricsResponse>) {
        let builder = self
            .client
            .get("https://lrclib.net/api/search")
            .header(
                "User-Agent",
                format!(
                    "Rei Music Downloader v{} (https://github.com/Zefirchiky/rei-music-downloader)",
                    crate::VERSION
                ),
            )
            .query(query);

        let req = builder.send().await.unwrap(); // FIXME: Remove unwrap

        (req.url().clone(), req.json().await.unwrap()) // FIXME: Same here
    }

    pub async fn search_with_track_artist(
        &self,
        artist: Option<&String>,
        track: &Track,
    ) -> (reqwest::Url, Vec<LyricsResponse>) {
        let mut query = vec![("track_name", track.as_str())];
        if let Some(a) = artist {
            query.push(("artist_name", a.as_str()));
        }
        self.search_with_query(&query).await
    }

    pub async fn search(&self, query: &str) -> (reqwest::Url, Vec<LyricsResponse>) {
        self.search_with_query(&[("q", query)]).await
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsResponse {
    pub plain_lyrics: Option<String>,
    pub synced_lyrics: Option<String>,
    pub artist_name: String,
    pub track_name: String,
    pub duration: f32,
    pub instrumental: bool,
}

impl Display for LyricsResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sync_hint = if self.synced_lyrics.is_some() {
            " [Sync]"
        } else {
            ""
        };
        let inst_hint = if self.instrumental {
            " [Instrumental]"
        } else {
            ""
        };
        let duration_min = (self.duration / 60.0).floor();
        let duration_sec = (self.duration % 60.0).round();

        write!(
            f,
            "{} - {} ({:0>2}:{:0>2}){}{}",
            self.artist_name, self.track_name, duration_min, duration_sec, sync_hint, inst_hint
        )
    }
}
