pub mod error;
pub mod formats;
pub mod speakers;

pub use self::error::WaveError;
pub use self::formats::Format;
pub use self::speakers::SpeakerPosition;

use memmap::Mmap;
use std::{
  fs::File,
  io::{Cursor, Seek, SeekFrom},
};

use byteorder::{LittleEndian, ReadBytesExt};

const RIFF: u32 = 0x46464952;
const WAVE: u32 = 0x45564157;
const FMT_: u32 = 0x20746d66;
const DATA: u32 = 0x61746164;
const LIST: u32 = 0x5453494c;
const FACT: u32 = 0x74636166;

/// Contains information included in the wavefile's header section,
/// describing the format, sample size, and number of audio channels
/// present.
#[derive(Debug, Copy, Clone)]
pub struct WaveInfo {
  /// Which encoding format this file uses.
  /// If the format is `Format::Extensible`, then the actual audio format is
  /// instead determined by the `subformat` field.
  pub audio_format: Format,
  /// Number of distinct audio channels.
  pub channels: u16,
  /// Number of audio samples per second.
  pub sample_rate: u32,
  /// Number of bytes per second of audio.
  pub byte_rate: u32,
  /// Number of bytes for one frame of audio.
  pub block_align: u16,
  /// Number of bits used to represent each sample.
  pub bits_per_sample: u16,
  /// Number of frames present in the file.  Each frame contains one sample per
  /// channel.
  pub total_frames: u32,
  /// Only present for `Format::Extensible` files.  Gives the actual number of
  /// valid bits per sample, which may be less than the value stored in
  /// `bits_per_sample`.
  pub valid_bps: Option<u16>,
  /// Only present for `Format::Extensible` files.  Contains a bit mask
  /// specifiying what speaker position to map each audio channel to.
  /// See `speakers()` for a parsed representation.
  pub channel_mask: Option<u32>,
  /// For `Format::Extensible` files, this field contains the actual audo encoding
  /// of the file, either `Format::PCM` or `Format::IEEEFloat`.
  pub subformat: Option<Format>,
}

pub struct WaveFile {
  mmap: Mmap,
  data_offset: u64,
  data_size: u32,
  info: WaveInfo,
}

/// An iterator which yields successive `Frames` of audio from the associated
/// wavefile.
pub struct WaveFileIterator<'a> {
  file: &'a WaveFile,
  pos: u64,
  base: u64,
  end: u64,
  bytes_per_sample: u16,
}

/// Represents a single frame of audio, containing one sample per audio channel.
/// For example, a mono audio file will contain only one sample; a stereo file
/// will contain two.
pub type Frame = Vec<i32>;

impl WaveFile {
  /// Constructs a new `WaveFile`.
  ///
  /// # Example
  ///
  /// ```
  /// use wavefile::{WaveFile,WaveError};
  ///
  /// match WaveFile::open("./fixtures/test-s24le.wav") {
  ///   Ok(f)  => f,
  ///   Err(e) => panic!("Couldn't open example file: {}", e)
  /// };
  /// ```
  pub fn open<S: Into<String>>(path: S) -> Result<WaveFile, WaveError> {
    let filename = path.into();
    let file = File::open(filename)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let info = WaveInfo {
      audio_format: Format::PCM,
      channels: 0,
      sample_rate: 0,
      byte_rate: 0,
      block_align: 0,
      bits_per_sample: 0,
      total_frames: 0,
      valid_bps: None,
      channel_mask: None,
      subformat: None,
    };
    let mut file = WaveFile {
      mmap: mmap,
      data_offset: 0,
      data_size: 0,
      info: info,
    };

    file.read_chunks()?;

    Ok(file)
  }

  /// The number of audio channels in the file.
  pub fn channels(&self) -> u16 {
    self.info.channels
  }

  /// The number of samples present for one second of audio.
  pub fn sample_rate(&self) -> u32 {
    self.info.sample_rate
  }

  /// The total number of frames present in the file.
  /// Each frame will contain `channels()` number of samples.
  pub fn len(&self) -> u32 {
    self.info.total_frames
  }

  /// The duration in milliseconds of the file.
  pub fn duration(&self) -> u32 {
    self.len() * 1000 / self.sample_rate()
  }

  pub fn bits_per_sample(&self) -> u16 {
    self.info.bits_per_sample
  }

  pub fn data_format(&self) -> Format {
    if self.info.audio_format == Format::Extensible {
      self.info.subformat.unwrap()
    } else {
      self.info.audio_format
    }
  }

  pub fn speakers(&self) -> Option<Vec<SpeakerPosition>> {
    match self.info.channel_mask {
      None => None,
      Some(mask) => Some(SpeakerPosition::decode(mask as isize)),
    }
  }

  /// Returns a copy of the `WaveInfo` for this file,
  /// parsed from the file header.
  pub fn info(&self) -> WaveInfo {
    self.info
  }

  /// Returns an iterator which yields each individual `Frame` successively
  /// until it reaches the end of the file.
  ///
  /// # Example
  ///
  /// ```no_run
  /// use wavefile::WaveFile;
  ///
  /// let wav = WaveFile::open("./fixtures/test-s24le.wav").unwrap();
  ///
  /// for frame in wav.iter() {
  ///   println!("{:?}", frame);
  /// }
  /// ```
  pub fn iter(&self) -> WaveFileIterator {
    let bytes_per_sample = self.info.bits_per_sample / 8;
    WaveFileIterator {
      file: &self,
      pos: 0,
      base: self.data_offset,
      end: self.data_offset + self.data_size as u64,
      bytes_per_sample: bytes_per_sample,
    }
  }

  fn read_format_chunk(info: &mut WaveInfo, cursor: &mut Cursor<&[u8]>) -> Result<(), WaveError> {
    let fmt = cursor.read_u16::<LittleEndian>()?;

    info.audio_format = match Format::decode(fmt) {
      Some(f) => f,
      None => {
        let msg = format!("Unexpected format {0:x}", fmt);
        return Err(WaveError::ParseError(msg));
      }
    };

    info.channels = cursor.read_u16::<LittleEndian>()?;
    info.sample_rate = cursor.read_u32::<LittleEndian>()?;
    info.byte_rate = cursor.read_u32::<LittleEndian>()?;
    info.block_align = cursor.read_u16::<LittleEndian>()?;
    info.bits_per_sample = cursor.read_u16::<LittleEndian>()?;

    if info.audio_format == Format::Extensible {
      match cursor.read_u16::<LittleEndian>()? {
        22 => {
          info.valid_bps = Some(cursor.read_u16::<LittleEndian>()?);
          info.channel_mask = Some(cursor.read_u32::<LittleEndian>()?);
          let subformat = cursor.read_u16::<LittleEndian>()?;
          info.subformat = match Format::decode(subformat) {
            Some(f) => Some(f),
            None => {
              let msg = format!("Unexpected subformat {0:x}", subformat);
              return Err(WaveError::ParseError(msg));
            }
          };
          cursor.seek(SeekFrom::Current(14))?;
        }
        x => {
          let msg = format!("Unexpected extension size: {}", x);
          return Err(WaveError::ParseError(msg));
        }
      }
    }

    Ok(())
  }

  fn read_chunks(&mut self) -> Result<(), WaveError> {
    let mut cursor = Cursor::new(self.mmap.as_ref());
    let mut have_fmt = false;
    let mut chunk_id = cursor.read_u32::<LittleEndian>()?;
    let mut chunk_size: u32;

    cursor.read_u32::<LittleEndian>()?;

    let riff_type = cursor.read_u32::<LittleEndian>()?;

    if chunk_id != RIFF || riff_type != WAVE {
      return Err(WaveError::ParseError("Not a Wavefile".into()));
    }

    loop {
      chunk_id = cursor.read_u32::<LittleEndian>()?;
      chunk_size = cursor.read_u32::<LittleEndian>()?;

      match chunk_id {
        FMT_ => {
          WaveFile::read_format_chunk(&mut self.info, &mut cursor)?;
          have_fmt = true;
        }
        DATA => {
          self.data_size = chunk_size;
          break;
        }
        LIST => {
          cursor.seek(SeekFrom::Current(chunk_size as i64))?;
        }
        FACT => {
          cursor.seek(SeekFrom::Current(chunk_size as i64))?;
        }
        other => {
          let msg = format!("Unexpected Chunk ID {0:x}", other);
          return Err(WaveError::ParseError(msg));
        }
      }
    }

    if !have_fmt {
      return Err(WaveError::ParseError("No format chunk found".into()));
    }

    self.validate_format()?;

    self.info.total_frames =
      self.data_size as u32 / (self.info.channels as u32 * self.info.bits_per_sample as u32 / 8);
    self.data_offset = cursor.position();

    Ok(())
  }

  fn validate_format(&self) -> Result<(), WaveError> {
    let bps = self.info.bits_per_sample;

    if self.info.channels == 0 {
      let msg = format!("No audio channels present in this file (weird, right?)");
      Err(WaveError::ParseError(msg))
    } else if self.info.bits_per_sample < 8 {
      let msg = format!("Unsupported bits per sample: {} expected at least 8.", bps);
      Err(WaveError::Unsupported(msg))
    } else if self.data_format() == Format::IEEEFloat && !(bps == 32 || bps == 64) {
      let msg = format!(
        "Unsupported bits per sample for floating point data: {} expected 32/64.",
        bps
      );
      Err(WaveError::Unsupported(msg))
    } else {
      Ok(())
    }
  }
}

impl<'a> Iterator for WaveFileIterator<'a> {
  type Item = Frame;

  fn next(&mut self) -> Option<Self::Item> {
    let mut cursor = Cursor::new(self.file.mmap.as_ref());

    if cursor.seek(SeekFrom::Start(self.base + self.pos)).is_err() {
      return None;
    };

    if cursor.position() == self.end {
      return None;
    }

    // TODO: if the data is in PCM format, we return the original values.
    // For example, pcm_8 yields values in the range [0, 255], while
    // pcm_16 yields values from [-32767, 32767].
    // However, for float data, since we can't return a float value here,
    // We convert the result to the full range of an i32.
    // Ideally we should let the caller specify what range they want the
    // data scaled to, if any;  however, I don't know how to do this without
    // writing out a million different conversion functions for each case.
    let (frame, new_pos) = match self.file.data_format() {
      Format::PCM => {
        WaveFileIterator::next_pcm(&mut cursor, self.file.channels(), self.bytes_per_sample)
      }
      Format::IEEEFloat => {
        WaveFileIterator::next_float(&mut cursor, self.file.channels(), self.bytes_per_sample)
      }
      _ => unreachable!(),
    };

    self.pos = new_pos - self.base;

    Some(frame)
  }
}

impl<'a> WaveFileIterator<'a> {
  fn next_pcm(cursor: &mut Cursor<&[u8]>, channels: u16, bps: u16) -> (Frame, u64) {
    let mut samples: Vec<i32> = Vec::with_capacity(channels as usize);

    for _ in 0..channels {
      match cursor.read_int::<LittleEndian>(bps as usize) {
        Ok(sample) => samples.push(sample as i32),
        Err(e) => {
          panic!("{:?}", e);
        }
      }
    }

    (samples, cursor.position())
  }

  fn next_float(cursor: &mut Cursor<&[u8]>, channels: u16, bps: u16) -> (Frame, u64) {
    match bps {
      4 => WaveFileIterator::next_float32(cursor, channels),
      8 => WaveFileIterator::next_float64(cursor, channels),
      _ => unreachable!(),
    }
  }

  fn next_float32(cursor: &mut Cursor<&[u8]>, channels: u16) -> (Frame, u64) {
    let mut samples: Vec<i32> = Vec::with_capacity(channels as usize);

    for _ in 0..channels {
      match cursor.read_f32::<LittleEndian>() {
        Ok(sample) => {
          let scaled = (sample * 2147483647.0) as i32;
          samples.push(scaled);
        }
        Err(e) => {
          panic!("{:?}", e);
        }
      }
    }

    (samples, cursor.position())
  }

  fn next_float64(cursor: &mut Cursor<&[u8]>, channels: u16) -> (Frame, u64) {
    let mut samples: Vec<i32> = Vec::with_capacity(channels as usize);

    for _ in 0..channels {
      match cursor.read_f64::<LittleEndian>() {
        Ok(sample) => {
          let scaled = (sample * 2147483647.0) as i32;
          samples.push(scaled);
        }
        Err(e) => {
          panic!("{:?}", e);
        }
      }
    }

    (samples, cursor.position())
  }
}

#[test]
fn test_info() {
  let file = match WaveFile::open("./fixtures/test-s24le.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e),
  };
  let info = file.info();

  assert_eq!(info.audio_format, Format::PCM);
  assert_eq!(info.channels, 2);
  assert_eq!(info.sample_rate, 48000);
  assert_eq!(info.byte_rate, 288000);
  assert_eq!(info.block_align, 6);
  assert_eq!(info.bits_per_sample, 24);
  assert_eq!(info.total_frames, 501888);

  let file = match WaveFile::open("./fixtures/test-u8.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e),
  };
  let info = file.info();

  assert_eq!(info.audio_format, Format::PCM);
  assert_eq!(info.channels, 2);
  assert_eq!(info.sample_rate, 48000);
  assert_eq!(info.byte_rate, 96000);
  assert_eq!(info.bits_per_sample, 8);
  assert_eq!(info.block_align, 2);
  assert_eq!(info.total_frames, 501888);
}

#[test]
fn test_iter() {
  let file = match WaveFile::open("./fixtures/test-s24le.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e),
  };

  let frames = file.iter().take(2).collect::<Vec<_>>();
  let expected = vec![[19581, 19581], [24337, 24337]];

  for i in 0..expected.len() {
    assert_eq!(frames[i], expected[i]);
  }

  let frame = file.iter().last().unwrap();
  let expected = [244, 244];

  assert_eq!(frame, expected)
}

#[test]
fn test_float_extensible() {
  let file = WaveFile::open("./fixtures/test-f32le.wav").unwrap();
  let info = file.info();

  assert_eq!(info.audio_format, Format::Extensible);
  assert_eq!(file.data_format(), Format::IEEEFloat);
  assert_eq!(file.len(), 501888);

  let frames = file.iter().take(2).collect::<Vec<_>>();
  // these are the same values as the 24-bit samples,
  // however we've scaled to 32-bit.
  let expected = vec![[5012736, 5012736], [6230272, 6230272]];

  for i in 0..expected.len() {
    assert_eq!(frames[i], expected[i]);
  }

  assert_eq!(
    file.speakers().unwrap(),
    [SpeakerPosition::FrontLeft, SpeakerPosition::FrontRight]
  );
}

#[test]
fn test_duration() {
  let file = WaveFile::open("./fixtures/test-s24le.wav").unwrap();
  assert_eq!(file.duration(), 10456);
}
