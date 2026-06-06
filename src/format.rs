#[derive(Clone, Copy)]
pub struct MinHeight(pub u32);

impl Default for MinHeight {
    fn default() -> Self {
        Self(720)
    }
}

pub enum Format {
    Merged,
    Split,
}

impl Format {
    pub fn format_string(&self, min_height: MinHeight) -> String {
        let min_height = min_height.0;
        match self {
            Format::Merged => format!("(mp4,webm)[height<={min_height}]"),
            Format::Split => format!(
                "bv[vcodec^=avc1][height<={min_height}]+ba[ext=m4a]/ba+bv[height<={min_height}]"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_height_defaults_to_720() {
        assert_eq!(MinHeight::default().0, 720);
    }

    #[test]
    fn merged_format_string_uses_max_height_constraint() {
        assert_eq!(
            Format::Merged.format_string(MinHeight::default()),
            "(mp4,webm)[height<=720]"
        );
        assert_eq!(
            Format::Merged.format_string(MinHeight(1080)),
            "(mp4,webm)[height<=1080]"
        );
    }

    #[test]
    fn split_format_string_uses_video_audio_fallbacks() {
        assert_eq!(
            Format::Split.format_string(MinHeight::default()),
            "bv[vcodec^=avc1][height<=720]+ba[ext=m4a]/ba+bv[height<=720]"
        );
        assert_eq!(
            Format::Split.format_string(MinHeight(480)),
            "bv[vcodec^=avc1][height<=480]+ba[ext=m4a]/ba+bv[height<=480]"
        );
    }
}
