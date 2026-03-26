use filess::Ogg;
use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, ItemKey, TagExt},
};

use crate::{Artist, Track, log_step};

pub struct Song {
    pub file: Ogg,
    pub artist: Artist,
    pub track: Track,
    pub url: String,
    pub lyrics: Option<String>,
}

impl Song {
    pub fn fix_metadata(&self) {
        let mut metadata = Probe::open(&self.file)
            .unwrap()
            .set_file_type(lofty::file::FileType::Opus)
            .read()
            .unwrap();
        let tag = metadata.primary_tag_mut().expect("No tags found");

        tag.remove_key(ItemKey::Description);
        tag.remove_key(ItemKey::Comment);

        tag.set_artist(self.artist.to_string());
        tag.set_title(self.track.to_string());
        tag.insert_text(ItemKey::AudioSourceUrl, self.url.clone());
        if let Some(lyr) = &self.lyrics {
            tag.insert_text(ItemKey::Lyrics, lyr.clone());
        }

        tag.save_to_path(&self.file, WriteOptions::default())
            .unwrap();
        log_step("Metadata was fixed");
    }
}
