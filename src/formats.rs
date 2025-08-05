const FORMAT_PCM: u16 = 1;
const FORMAT_IEEE: u16 = 3;
const FORMAT_EXT: u16 = 0xfffe;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Format {
  PCM = FORMAT_PCM as isize,
  IEEEFloat = FORMAT_IEEE as isize,
  Extensible = FORMAT_EXT as isize,
}

impl Format {
  pub fn decode(val: u16) -> Option<Format> {
    match val {
      FORMAT_PCM => Some(Format::PCM),
      FORMAT_IEEE => Some(Format::IEEEFloat),
      FORMAT_EXT => Some(Format::Extensible),
      _ => None,
    }
  }
}
