use regex::Regex;

use crate::components::{Artist, Component, Remix, RemixArtist, Track};

const SEPARATOR_REGEX: &str = r"(?i)\s*(?:[,+&\\|;]|\bx\b|\band\b)\s*";
const BRACKET_TAG_REGEX: &str = r"[\(\[](.*?)[\)\]]";

#[derive(Debug, PartialEq)]
pub struct ParsedTrack {
    pub artists: Vec<Artist>,
    pub remix_artists: Vec<RemixArtist>,
    pub tracks: Vec<Track>,
    pub remixes: Vec<Remix>,
}

impl ParsedTrack {
    pub fn new(
        artists: Vec<Artist>,
        remix_artists: Vec<RemixArtist>,
        tracks: Vec<Track>,
        remixes: Vec<Remix>,
    ) -> Self {
        let mut s = Self {
            artists,
            remix_artists,
            tracks,
            remixes,
        };
        s.dedup();
        s
    }

    pub fn dedup(&mut self) {
        self.artists.dedup();
        self.remix_artists.dedup();
        self.tracks.dedup();
        self.remixes.dedup();
    }
}

impl std::ops::Add for ParsedTrack {
    type Output = ParsedTrack;
    fn add(mut self, mut rhs: Self) -> Self::Output {
        self.artists.append(&mut rhs.artists);
        self.remix_artists.append(&mut rhs.remix_artists);
        self.tracks.append(&mut rhs.tracks);
        self.remixes.append(&mut rhs.remixes);
        self.dedup();
        self
    }
}

pub struct Parser {}

impl Parser {
    pub fn parse_title(title: String) -> Option<ParsedTrack> {
        if title.is_empty() {
            return None;
        }

        Some(ParsedTrack::new(
            Self::extract_artists(&title),
            Self::extract_remix_artists(&title),
            Self::extract_titles(&title),
            Self::extract_remixes(&title),
        ))
    }

    /// Helper function to split artist lists on common delimiters: ',', '&', or ' x '
    fn split_list<T: Component>(text: &str) -> Vec<T> {
        let delim_re = Regex::new(SEPARATOR_REGEX).unwrap();
        delim_re
            .split(text)
            .map(|s| T::from(s))
            .filter(|a| !a.as_ref().is_empty())
            .collect()
    }

    /// EXTRACT artists
    /// Takes all sections separated by " - " EXCEPT the last section,
    /// then splits each section by artist delimiters (',', '&', ' x ').
    pub fn extract_artists(input: &str) -> Vec<Artist> {
        let main_dash_re = Regex::new(r"\s+-\s+").unwrap();
        let parts: Vec<&str> = main_dash_re.split(input).collect();

        if parts.len() <= 1 {
            return Vec::new();
        }

        // Take all sections except the last one (which contains song name & tags)
        let artist_sections = &parts[..parts.len() - 1];

        let mut artists: Vec<Artist> = artist_sections
            .iter()
            .flat_map(|section| Self::split_list(section))
            .collect();

        artists.dedup();
        artists
    }

    /// EXTRACT REMIX artists
    /// Finds parenthesized/bracketed groups containing `feat.`, `ft.`, or `... Remix`
    pub fn extract_remix_artists(input: &str) -> Vec<RemixArtist> {
        // Matches content inside () or []
        let tag_re = Regex::new(BRACKET_TAG_REGEX).unwrap();
        // Matches "feat. X", "ft. X", or "X Remix"
        let ft_re = Regex::new(r"(?i)^(?:feat\.?|ft\.?)\s*(.+)$").unwrap();
        let remix_artist_re = Regex::new(r"(?i)^(.+?)\s+remix$").unwrap();

        let mut remix_artists = Vec::new();

        for cap in tag_re.captures_iter(input) {
            let content = cap[1].trim();

            if let Some(ft_cap) = ft_re.captures(content) {
                remix_artists.extend(Self::split_list(&ft_cap[1]));
            } else if let Some(rmx_cap) = remix_artist_re.captures(content) {
                remix_artists.extend(Self::split_list(&rmx_cap[1]));
            }
        }

        remix_artists.dedup();
        remix_artists
    }

    /// EXTRACT REMIXES / TAGS
    /// Matches audio edits like "Slowed", "Reverb", "Sped Up", "Nightcore", etc.
    pub fn extract_remixes(input: &str) -> Vec<Remix> {
        let tag_re = Regex::new(BRACKET_TAG_REGEX).unwrap();
        // Sub-split tags separated by '&', '+', or ',' inside brackets

        // Comprehensive list of common remix/audio-style keywords
        let edit_keywords = [
            "slowed",
            "over slowed",
            "reverb",
            "sped up",
            "over sped up",
            "spedup",
            "over spedup",
            "nightcore",
            "daycore",
            "8d",
            "8d audio",
            "instrumental",
            "acoustic",
            "bass boosted",
            "vip",
            "extended mix",
            "club mix",
            "edit",
            "radio edit",
        ];

        let mut remixes = Vec::new();

        for cap in tag_re.captures_iter(input) {
            let content = cap[1].trim();

            // Ignore featuring tags
            if content.to_lowercase().starts_with("feat")
                || content.to_lowercase().starts_with("ft")
            {
                continue;
            }

            // Split tags like "Slowed & Reverb" into ["Slowed", "Reverb"]
            for sub_tag in Self::split_list::<Remix>(content) {
                let lower_tag = sub_tag.to_lowercase();

                if edit_keywords.iter().any(|&k| lower_tag.contains(k)) {
                    remixes.push(Remix::new(&lower_tag));
                }
            }
        }

        remixes.dedup();
        remixes
    }

    /// EXTRACT SONG NAME
    /// Extracts the last main segment before tags, stripping away () and [] metadata.
    pub fn extract_titles(input: &str) -> Vec<Track> {
        let main_dash_re = Regex::new(r"\s+-\s+").unwrap();
        let parts: Vec<&str> = main_dash_re.split(input).collect();

        // Take all sections except the first artist section (or fall back to input if no dash)
        let raw_name_sections = if parts.len() > 1 {
            &parts[1..]
        } else {
            &parts[..]
        };

        let strip_tags_re = Regex::new(r"\s*[\(\[][^\)\]]*[\)\]]").unwrap();

        let mut names: Vec<Track> = raw_name_sections
            .iter()
            .map(|section| {
                // Strip out bracketed and parenthesized metadata
                let cleaned = strip_tags_re.replace_all(section, "");
                Track::new(&cleaned.trim())
            })
            .filter(|t| !t.is_empty())
            .collect();

        names.dedup();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_artists() {
        let input = "NoCopyrightSounds - artist1 & artist 2, artist 3 - name of the song";
        let artists = Parser::extract_artists(input);
        assert_eq!(
            artists,
            vec!["NoCopyrightSounds", "artist1", "artist 2", "artist 3"]
        );
    }

    #[test]
    fn extract_artists_dedup() {
        let input = "NoCopyrightSounds - artist1 & artist 2, artist 2, artist 3 - name of the song";
        let artists = Parser::extract_artists(input);
        assert_eq!(
            artists,
            vec!["NoCopyrightSounds", "artist1", "artist 2", "artist 3"]
        );
    }

    #[test]
    fn extract_remix_artists() {
        let input1 = "Artist - Song (feat. artist4 & artist 5) [Slowed]";
        let input2 = "Artist - Song (ft. artist4) (artist 5 Remix)";

        assert_eq!(
            Parser::extract_remix_artists(input1),
            vec!["artist4", "artist 5"]
        );
        assert_eq!(
            Parser::extract_remix_artists(input2),
            vec!["artist4", "artist 5"]
        );
    }

    #[test]
    fn extract_remixes() {
        let input = "Artist - Song (feat. artist) [Slowed & Reverb] (Sped Up)";
        let remixes = Parser::extract_remixes(input);
        assert_eq!(remixes, vec!["slowed", "reverb", "sped up"]);
    }

    #[test]
    fn extract_names() {
        let input =
            "NoCopyrightSounds - artist1 & artist 2, artist 3 - name of the song (feat. artist4)";
        let names = Parser::extract_titles(input);

        assert_eq!(
            names,
            vec!["artist1 & artist 2, artist 3", "name of the song",]
        );
    }

    #[test]
    fn parse_title_example_1() {
        let input = "NoCopyrightSounds - artist1 & artist 2, artist 3 - name of the song (feat. artist4 & artist 5) [Slowed] (Reverb)";
        let result = Parser::parse_title(input.into());

        assert!(result.is_some());
        let parsed = result.unwrap();

        assert_eq!(
            parsed.artists,
            vec!["NoCopyrightSounds", "artist1", "artist 2", "artist 3"]
        );
        assert_eq!(parsed.remix_artists, vec!["artist4", "artist 5"]);
        assert_eq!(
            parsed.tracks,
            vec!["artist1 & artist 2, artist 3", "name of the song",]
        );
        assert_eq!(parsed.remixes, vec!["slowed", "reverb"]);
    }

    #[test]
    fn parse_title_example_2() {
        let input =
            "artist1 x artist 2, & artist 3 - name of the song (ft. artist4) [Slowed & Reverb]";
        let result = Parser::parse_title(input.into());

        assert!(result.is_some());
        let parsed = result.unwrap();

        assert_eq!(parsed.artists, vec!["artist1", "artist 2", "artist 3"]);
        assert_eq!(parsed.remix_artists, vec!["artist4"]);
        assert_eq!(parsed.tracks, vec!["name of the song"]);
        assert_eq!(parsed.remixes, vec!["slowed", "reverb"]);
    }

    #[test]
    fn parse_title_invalid_format() {
        let input = "";
        let result = Parser::parse_title(input.into());
        assert_eq!(result, None);
    }
}
