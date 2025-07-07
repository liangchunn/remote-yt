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
    pub fn get_format_string(&self, min_height: MinHeight) -> String {
        let min_height = min_height.0;
        match self {
            Format::Merged => format!("(mp4,webm)[height<={min_height}]"),
            Format::Split => format!(
                "bv[vcodec^=avc1][height<={min_height}]+ba[ext=m4a]/ba+bv[height<={min_height}]"
            ),
        }
    }
}
