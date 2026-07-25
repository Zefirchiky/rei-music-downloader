use std::collections::HashSet;

use filess::{Ogg, traits::FileTrait};
use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, ItemKey, ItemValue, Tag, TagExt, TagItem},
};

use crate::{
    components::{Artist, Remix, RemixArtist, Title}, parser::{ParsedTrack, Parser},
};

#[derive(Debug)]
pub struct Song {
    pub file: Ogg,
    pub artists: Vec<Artist>,
    pub remix_artists: Vec<RemixArtist>,
    pub title: Title,
    pub remixes: Vec<Remix>,
    pub url: String,
    pub lyrics: Option<String>,
}

impl Song {
    pub fn fix(song: &Ogg) -> Song {
        let mut parsed = Parser::parse_title(song.file_stem().unwrap().to_str().unwrap().to_string())
            .expect("Can't be empty, enforced by filess");
        let mut song = Self::from_metadata(song.clone());

        let mut song_artists = vec![];
        for artist in &song.artists {
            song_artists.extend(Parser::split_list(artist))
        }
        song.artists = song_artists;

        let mut song_remix_artists = vec![];
        for artist in &song.remix_artists {
            song_remix_artists.extend(Parser::split_list(artist))
        }
        song.remix_artists = song_remix_artists;

        if !song.title.is_empty() {
            parsed = parsed + Parser::parse_title(song.title.0.clone()).unwrap();
        }

        let song_has_remix = !song.remix_artists.is_empty();
        let parsed_has_remix = !parsed.remix_artists.is_empty();

        // This part was made with ai cus I'm way to lazy for this shit
        // Step 1: Resolve remix_artists
        match (song_has_remix, parsed_has_remix) {
            // Case 1: Song doesn't have remix_artists, parsed does
            (false, true) => {
                let mut resolved_remix = Vec::new();

                for p_remix in parsed.remix_artists {
                    let p_lower = p_remix.to_lowercase();
                    // If the remix artist exists in song.artists, take song's representation
                    if let Some(pos) = song.artists.iter().position(|a| a.to_lowercase() == p_lower) {
                        let matched_artist = song.artists.remove(pos);
                        resolved_remix.push(RemixArtist(matched_artist.0));
                    } else {
                        resolved_remix.push(p_remix);
                    }
                }
                song.remix_artists = resolved_remix;
            }
            // Case 4: Both have remix_artists -> Combine & dedup (song preferred)
            (true, true) => {
                let mut seen = HashSet::new();
                let mut combined_remix = Vec::new();

                for remix in song.remix_artists.drain(..) {
                    if seen.insert(remix.to_lowercase()) {
                        combined_remix.push(remix);
                    }
                }
                for remix in parsed.remix_artists {
                    if seen.insert(remix.to_lowercase()) {
                        combined_remix.push(remix);
                    }
                }
                song.remix_artists = combined_remix;
            }
            // Case 2 & 3: Song has remix (or neither does) -> keep song.remix_artists as-is
            (true, false) | (false, false) => {}
        }

        // Step 2: Resolve artists
        // Build a set of all lowercase remix_artists to filter out from regular artists
        let remix_set: HashSet<String> = song
            .remix_artists
            .iter()
            .map(|r| r.to_lowercase())
            .collect();

        let mut seen_artists = HashSet::new();
        let mut combined_artists = Vec::new();

        // Add song's remaining artists first (Preference to song)
        for artist in song.artists.drain(..) {
            let lower = artist.to_lowercase();
            if !remix_set.contains(&lower) && seen_artists.insert(lower) {
                combined_artists.push(artist);
            }
        }

        // Add parsed's artists second
        for artist in parsed.artists {
            let lower = artist.to_lowercase();
            if !remix_set.contains(&lower) && seen_artists.insert(lower) {
                combined_artists.push(artist);
            }
        }

        song.artists = combined_artists;
        song.title = parsed.tracks[0].clone();

        let mut remixes = song.remixes.clone();
        remixes.append(&mut parsed.remixes);
        song.remixes = ParsedTrack::dedup_one(remixes);
        
        // dbg!(&song);
        // if song.file.file_stem().unwrap().to_str().unwrap().to_string().contains("Vertigo") {
        // }
        song.fix_metadata();

        let mut full_artist = song.artists.iter().map(|a| a.0.clone()).collect::<Vec<_>>();
        full_artist.extend(song.remix_artists.iter().map(|a| a.0.clone()));
        let full_artist_string = full_artist.join(", ");    // FIXME: Customization
        
        let mut remixes_string = "".to_string();
        if !song.remixes.is_empty() {
            remixes_string = " [".to_string()
                + &song
                    .remixes
                    .iter()
                    .map(|r| r.to_uppercase())
                    .collect::<Vec<String>>()
                    .join(" & ")
                + "]";
        }
        song.file.rename_file(format!("{full_artist_string} - {}{remixes_string}.opus", song.title)).unwrap();  // FIXME: This should be abstracted
        song
    }

    pub fn push_tags(tag: &mut Tag, key: ItemKey, elements: &[impl AsRef<str>]) {
        tag.remove_key(key);
        for el in elements {
            tag.push(TagItem::new(key, ItemValue::Text(el.as_ref().to_string())));
        }
    }

    pub fn from_metadata(song: Ogg) -> Self {
        let mut metadata = Probe::open(&song)
            .unwrap()
            .set_file_type(lofty::file::FileType::Opus)
            .read()
            .unwrap();
        let tag = metadata.primary_tag_mut().expect("No tags found");

        let producers = tag.get_strings(ItemKey::Producer).collect::<Vec<_>>();

        // If producers is empty, song is not a remix, and `TrackArtist` is original artists
        // Otherwise, `Producer` is original artists, while `TrackArtist` are remixers
        let mut remix_artists = vec![];
        let artists = if producers.is_empty() {
            tag.get_strings(ItemKey::TrackArtist)
                .map(Artist::from)
                .collect()
        } else {
            remix_artists = tag.get_strings(ItemKey::TrackArtist).map(RemixArtist::from).collect::<Vec<_>>();
            producers.iter().map(Artist::new).collect()
        };

        let remixes = tag
            .get_strings(ItemKey::Remixer)
            .map(Remix::from)
            .collect::<Vec<_>>();
        let title = Title::from(tag
            .get_string(ItemKey::TrackTitle)
            .unwrap_or(""));
        let url = String::from(tag
            .get_string(ItemKey::AudioSourceUrl)
            .unwrap_or(""));

        let lyrics = tag.get_strings(ItemKey::Lyrics).map(String::from).collect::<Vec<_>>().join("\n");

        Self {
            file: song,
            artists,
            remix_artists,
            title,
            remixes,
            url,
            lyrics: if !lyrics.is_empty() { Some(lyrics) } else { None },
        }
    }

    pub fn fix_metadata(&self) {
        let mut metadata = Probe::open(&self.file)
            .unwrap()
            .set_file_type(lofty::file::FileType::Opus)
            .read()
            .unwrap();
        let tag = metadata.primary_tag_mut().expect("No tags found");

        tag.remove_key(ItemKey::Description);
        tag.remove_key(ItemKey::Comment);

        tag.set_title(self.title.0.clone());
        if self.remix_artists.is_empty() {
            Self::push_tags(tag, ItemKey::TrackArtist, &self.artists);
        } else {
            Self::push_tags(
                tag,
                ItemKey::from_key(lofty::tag::TagType::VorbisComments, "PERFORMER").unwrap(),
                &self.artists,
            );
            Self::push_tags(tag, ItemKey::TrackArtist, &self.remix_artists);
        }
        Self::push_tags(
            tag,
            ItemKey::Remixer,
            &self
                .remixes
                .iter()
                .map(|r| r.to_uppercase())
                .collect::<Vec<_>>(),
        );

        if let Some(lyr) = &self.lyrics {
            tag.remove_key(ItemKey::Lyrics);
            tag.push(TagItem::new(ItemKey::Lyrics, ItemValue::Text(lyr.clone())));
        }

        tag.insert_text(
            ItemKey::Description,
            format!("File created by rei-music-downloader v{}", crate::VERSION),
        );
        tag.insert_text(ItemKey::AudioSourceUrl, self.url.clone());

        tag.save_to_path(&self.file, WriteOptions::default())
            .unwrap();
    }
}
