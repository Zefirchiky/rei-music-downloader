use std::fmt::Display;

use reqwest::Client;
use serde::Deserialize;

use crate::{Artist, Track};

#[derive(Default)]
pub struct Lyrics {
    client: Client,
}

impl Lyrics {
    pub async fn search(
        &self,
        artist: Option<&Artist>,
        track: &Track,
    ) -> (reqwest::Url, Vec<LyricsResponse>) {
        let mut builder = self
            .client
            .get("https://lrclib.net/api/search")
            .query(&[("track_name", track.as_str())]);
        if let Some(a) = artist {
            builder = builder.query(&[("artist_name", a.as_str())]);
        }
        let req = builder.send().await.unwrap();

        (req.url().clone(), req.json().await.unwrap())
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
