use filess::Ogg;
use lofty::{
    config::WriteOptions, file::TaggedFileExt, probe::Probe, tag::{Accessor, ItemKey, ItemValue, Tag, TagExt, TagItem},
};

use crate::{RemixArtist, Track, components::{Artist, Remix}, log_step};

pub struct Song {
    pub file: Ogg,
    pub artists: Vec<Artist>,
    pub remix_artists: Vec<RemixArtist>,
    pub track: Track,
    pub remixes: Vec<Remix>,
    pub url: String,
    pub lyrics: Option<String>,
}

impl Song {
    pub fn push_tags(tag: &mut Tag, key: ItemKey, elements: &[impl AsRef<str>]) {
        tag.remove_key(key);
        for el in elements {
            tag.push(TagItem::new(key, ItemValue::Text(el.as_ref().to_string())));
        }
    }
    
    pub fn add_from_metadata(&mut self) {
        let mut metadata = Probe::open(&self.file)
            .unwrap()
            .set_file_type(lofty::file::FileType::Opus)
            .read()
            .unwrap();
        let tag = metadata.primary_tag_mut().expect("No tags found");

        #[allow(unused)]
        tag.get_strings(ItemKey::TrackArtist).map(|s| self.artists.push(s.into()));
        #[allow(unused)]
        tag.get_strings(ItemKey::Remixer).map(|s| self.remix_artists.push(s.into()));
        if let Some(title) = tag.get_string(ItemKey::TrackTitle) {
            self.track = title.into();
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

        
        tag.set_title(self.track.as_ref().to_string());
        if self.remix_artists.is_empty() {
            Self::push_tags(tag, ItemKey::TrackArtist, &self.artists);
        } else {
            Self::push_tags(tag, ItemKey::from_key(lofty::tag::TagType::VorbisComments, "PERFORMER").unwrap(), &self.artists);
            Self::push_tags(tag, ItemKey::TrackArtist, &self.remix_artists);
        }
        Self::push_tags(tag, ItemKey::Remixer, &self.remixes.iter().map(|r| r.to_uppercase()).collect::<Vec<_>>());
        
        if let Some(lyr) = &self.lyrics {
            tag.remove_key(ItemKey::Lyrics);
            tag.push(TagItem::new(ItemKey::Lyrics, ItemValue::Text(lyr.clone())));
        }

        tag.insert_text(ItemKey::Description, format!("File created by rei-music-downloader v{}", crate::VERSION));
        tag.insert_text(ItemKey::AudioSourceUrl, self.url.clone());

        tag.save_to_path(&self.file, WriteOptions::default())
            .unwrap();
        log_step("Metadata was fixed");
    }
}
