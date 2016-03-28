extern crate memmap;
extern crate byteorder;

pub mod error;

pub use self::error::WaveError;

use std::io::{Seek,SeekFrom,Cursor};
use memmap::{Mmap,Protection};

use byteorder::{LittleEndian, ReadBytesExt};

const RIFF : u32 = 0x46464952;
const WAVE : u32 = 0x45564157;
const FMT_ : u32 = 0x20746d66;
const DATA : u32 = 0x61746164;
const LIST : u32 = 0x5453494c;
const FACT : u32 = 0x74636166;

const FORMAT_PCM  : u16 = 1;
const FORMAT_IEEE : u16 = 3;
const FORMAT_EXT  : u16 = 0xfffe;

const SPEAKER_FRONT_LEFT            : isize = (1 << 0);
const SPEAKER_FRONT_RIGHT           : isize = (1 << 1);
const SPEAKER_FRONT_CENTER          : isize = (1 << 2);
const SPEAKER_LOW_FREQUENCY         : isize = (1 << 3);
const SPEAKER_BACK_LEFT             : isize = (1 << 4);
const SPEAKER_BACK_RIGHT            : isize = (1 << 5);
const SPEAKER_FRONT_LEFT_OF_CENTER  : isize = (1 << 6);
const SPEAKER_FRONT_RIGHT_OF_CENTER : isize = (1 << 7);
const SPEAKER_BACK_CENTER           : isize = (1 << 8);
const SPEAKER_SIDE_LEFT             : isize = (1 << 9);
const SPEAKER_SIDE_RIGHT            : isize = (1 << 10);
const SPEAKER_TOP_CENTER            : isize = (1 << 11);
const SPEAKER_TOP_FRONT_LEFT        : isize = (1 << 12);
const SPEAKER_TOP_FRONT_CENTER      : isize = (1 << 13);
const SPEAKER_TOP_FRONT_RIGHT       : isize = (1 << 14);
const SPEAKER_TOP_BACK_LEFT         : isize = (1 << 15);
const SPEAKER_TOP_BACK_CENTER       : isize = (1 << 16);
const SPEAKER_TOP_BACK_RIGHT        : isize = (1 << 17);
const SPEAKER_RESERVED              : isize = (1 << 31);

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum SpeakerPosition {
  FrontLeft          = SPEAKER_FRONT_LEFT,
  FrontRight         = SPEAKER_FRONT_RIGHT,
  FrontCenter        = SPEAKER_FRONT_CENTER,
  LowFrequency       = SPEAKER_LOW_FREQUENCY,
  BackLeft           = SPEAKER_BACK_LEFT,
  BackRight          = SPEAKER_BACK_RIGHT,
  FrontLeftOfCenter  = SPEAKER_FRONT_LEFT_OF_CENTER,
  FrontRightOfCenter = SPEAKER_FRONT_RIGHT_OF_CENTER,
  BackCenter         = SPEAKER_BACK_CENTER,
  SideLeft           = SPEAKER_SIDE_LEFT,
  SideRight          = SPEAKER_SIDE_RIGHT,
  TopCenter          = SPEAKER_TOP_CENTER,
  TopFrontLeft       = SPEAKER_TOP_FRONT_LEFT,
  TopFrontCenter     = SPEAKER_TOP_FRONT_CENTER,
  TopFrontRight      = SPEAKER_TOP_FRONT_RIGHT,
  TopBackLeft        = SPEAKER_TOP_BACK_LEFT,
  TopBackCenter      = SPEAKER_TOP_BACK_CENTER,
  TopBackRight       = SPEAKER_TOP_BACK_RIGHT,
  Reserved           = SPEAKER_RESERVED,
}

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Format {
  PCM       = FORMAT_PCM  as isize,
  IEEEFloat = FORMAT_IEEE as isize,
  Extended  = FORMAT_EXT  as isize
}

/// Contains information included in the wavefile's header section,
/// describing the format, sample size, and number of audio channels
/// present.
#[derive(Debug,Copy,Clone)]
pub struct WaveInfo {
  /// Which encoding format this file uses.
  /// If the format is `Format::Extended`, then the actual audio format is
  /// instead determined by the `subformat` field.
  pub audio_format:    Format,
  /// Number of distinct audio channels.
  pub channels:        u16,
  /// Number of audio samples per second.
  pub sample_rate:     u32,
  pub byte_rate:       u32,
  pub block_align:     u16,
  /// Number of bits used to represent each sample.
  pub bits_per_sample: u16,
  /// Number of frames present in the file.  Each frame contains one sample per
  /// channel.
  pub total_frames:    u32,
  pub valid_bps:       Option<u16>,
  pub channel_mask:    Option<u32>,
  /// For `Format::Extended` files, this field contains the actual audo encoding
  /// of the file, either `Format::PCM` or `Format::IEEEFloat`.
  pub subformat:       Option<Format>
}

pub struct WaveFile {
  mmap:        Mmap,
  data_offset: usize,
  data_size:   usize,
  info:        WaveInfo
}

/// An iterator which yields successive `Frames` of audio from the associated
/// wavefile.
pub struct WaveFileIterator<'a> {
  file:             &'a WaveFile,
  pos:              usize,
  base:             usize,
  end:              usize,
  bytes_per_sample: usize,
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
    let mmap = try!(Mmap::open_path(filename, Protection::Read));
    let info = WaveInfo {
      audio_format:    Format::PCM,
      channels:        0,
      sample_rate:     0,
      byte_rate:       0,
      block_align:     0,
      bits_per_sample: 0,
      total_frames:    0,
      valid_bps:       None,
      channel_mask:    None,
      subformat:       None
    };
    let mut file = WaveFile { mmap: mmap, data_offset: 0, data_size: 0, info: info };

    try!(file.read_header_chunks());

    Ok(file)
  }

  /// The number of audio channels in the file.
  pub fn channels(&self) -> usize {
    self.info.channels as usize
  }

  /// The number of samples present for one second of audio.
  pub fn sample_rate(&self) -> usize {
    self.info.sample_rate as usize
  }

  /// The total number of frames present in the file.
  /// Each frame will contain `channels()` number of samples.
  pub fn len(&self) -> usize {
    self.info.total_frames as usize
  }

  pub fn bits_per_sample(&self) -> usize {
    self.info.bits_per_sample as usize
  }

  pub fn data_format(&self) -> Format {
    if self.info.audio_format == Format::Extended {
      self.info.subformat.unwrap()
    } else {
      self.info.audio_format
    }
  }

  pub fn speakers(&self) -> Option<Vec<SpeakerPosition>> {
    match self.info.channel_mask {
      None => None,
      Some(mask) => {
        let mask = mask as isize;
        let mut speakers = vec![];
        let mut i = SPEAKER_FRONT_LEFT;
        while i < SPEAKER_RESERVED {
          if mask & i != 0 {
            speakers.push(match i {
              SPEAKER_FRONT_LEFT            => SpeakerPosition::FrontLeft,
              SPEAKER_FRONT_RIGHT           => SpeakerPosition::FrontRight,
              SPEAKER_FRONT_CENTER          => SpeakerPosition::FrontCenter,
              SPEAKER_LOW_FREQUENCY         => SpeakerPosition::LowFrequency,
              SPEAKER_BACK_LEFT             => SpeakerPosition::BackLeft,
              SPEAKER_BACK_RIGHT            => SpeakerPosition::BackRight,
              SPEAKER_FRONT_LEFT_OF_CENTER  => SpeakerPosition::FrontLeftOfCenter,
              SPEAKER_FRONT_RIGHT_OF_CENTER => SpeakerPosition::FrontRightOfCenter,
              SPEAKER_BACK_CENTER           => SpeakerPosition::BackCenter,
              SPEAKER_SIDE_LEFT             => SpeakerPosition::SideLeft,
              SPEAKER_SIDE_RIGHT            => SpeakerPosition::SideRight,
              SPEAKER_TOP_CENTER            => SpeakerPosition::TopCenter,
              SPEAKER_TOP_FRONT_LEFT        => SpeakerPosition::TopFrontLeft,
              SPEAKER_TOP_FRONT_CENTER      => SpeakerPosition::TopFrontCenter,
              SPEAKER_TOP_FRONT_RIGHT       => SpeakerPosition::TopFrontRight,
              SPEAKER_TOP_BACK_LEFT         => SpeakerPosition::TopBackLeft,
              SPEAKER_TOP_BACK_CENTER       => SpeakerPosition::TopBackCenter,
              SPEAKER_TOP_BACK_RIGHT        => SpeakerPosition::TopBackRight,
              _                             => unreachable!()
            });
          }
          i <<= 1;
        }
        Some(speakers)
      }
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
    let bytes_per_sample = self.info.bits_per_sample as usize / 8;
    WaveFileIterator {
      file:             &self,
      pos:              0,
      base:             self.data_offset,
      end:              self.data_offset + self.data_size,
      bytes_per_sample: bytes_per_sample
    }
  }

  fn read_header_chunks(&mut self) -> Result<(), WaveError> {
    let mut cursor   = Cursor::new(unsafe { self.mmap.as_slice() } );
    let mut have_fmt = false;
    let mut chunk_id = try!(cursor.read_u32::<LittleEndian>());

    let mut chunk_size : u32;

    try!(cursor.read_u32::<LittleEndian>());

    let riff_type = try!(cursor.read_u32::<LittleEndian>());

    if chunk_id != RIFF || riff_type != WAVE {
      return Err(WaveError::ParseError("Not a Wavefile".into()));
    }


    loop {
      chunk_id   = try!(cursor.read_u32::<LittleEndian>());
      chunk_size = try!(cursor.read_u32::<LittleEndian>());

      match chunk_id {
        FMT_ => {
          have_fmt = true;
          self.info.audio_format = match try!(cursor.read_u16::<LittleEndian>()) {
            FORMAT_PCM  => Format::PCM,
            FORMAT_IEEE => Format::IEEEFloat,
            FORMAT_EXT  => Format::Extended,
            other       => {
              let msg = format!("Unexpected format {0:x}", other);
              return Err(WaveError::ParseError(msg));
            }
          };
          self.info.channels        = try!(cursor.read_u16::<LittleEndian>());
          self.info.sample_rate     = try!(cursor.read_u32::<LittleEndian>());
          self.info.byte_rate       = try!(cursor.read_u32::<LittleEndian>());
          self.info.block_align     = try!(cursor.read_u16::<LittleEndian>());
          self.info.bits_per_sample = try!(cursor.read_u16::<LittleEndian>());

          if self.info.audio_format == Format::Extended {
            match try!(cursor.read_u16::<LittleEndian>()) {
              0 => { },
              22 => {
                self.info.valid_bps    = Some(try!(cursor.read_u16::<LittleEndian>()));
                self.info.channel_mask = Some(try!(cursor.read_u32::<LittleEndian>()));
                self.info.subformat    = match try!(cursor.read_u16::<LittleEndian>()) {
                  FORMAT_PCM  => Some(Format::PCM),
                  FORMAT_IEEE => Some(Format::IEEEFloat),
                  other       => {
                    let msg = format!("Unexpected subformat {0:x}", other);
                    return Err(WaveError::ParseError(msg));
                  }
                };
                try!(cursor.seek(SeekFrom::Current(14)));
              },
              x => {
                let msg = format!("Unexpected extension size: {}", x);
                return Err(WaveError::ParseError(msg));
              }
            }

          }
        },
        DATA  => {
          self.data_size = chunk_size as usize;
          break;
        },
        LIST  => { try!(cursor.seek(SeekFrom::Current(chunk_size as i64))); },
        FACT  => { try!(cursor.seek(SeekFrom::Current(chunk_size as i64))); },
        other => {
          let msg = format!("Unexpected Chunk ID {0:x}", other);
          return Err(WaveError::ParseError(msg));
        }
      }
    }

    if !have_fmt {
      return Err(WaveError::ParseError("Format Chunk not found".into()));
    }

    if self.info.channels == 0 || self.info.bits_per_sample < 8 {
      let msg = format!("Invalid channel count {} or bits per sample {} value",
                        self.info.channels, self.info.bits_per_sample);

      return Err(WaveError::ParseError(msg));
    }

    self.info.total_frames = self.data_size as u32 / (self.info.channels as u32 * self.info.bits_per_sample as u32 / 8 );

    self.data_offset = cursor.position() as usize;
    Ok(())
  }
}

impl<'a> Iterator for WaveFileIterator<'a> {
  type Item = Frame;

  fn next(&mut self) -> Option<Self::Item> {
    let mut cursor = Cursor::new(unsafe { self.file.mmap.as_slice() });

    if let Err(_) = cursor.seek(SeekFrom::Start((self.base + self.pos) as u64)) {
      return None;
    };

    if cursor.position() as usize == self.end {
      return None;
    }

    let (frame, new_pos) = match self.file.data_format() {
      Format::PCM => WaveFileIterator::next_pcm(&mut cursor,
                                                self.file.channels(),
                                                self.bytes_per_sample),
      Format::IEEEFloat => WaveFileIterator::next_float(&mut cursor,
                                                        self.file.channels(),
                                                        self.bytes_per_sample),
      _ => unreachable!()
    };

    self.pos = new_pos - self.base;


    Some(frame)
  }
}

impl<'a> WaveFileIterator<'a> {
  fn next_pcm(cursor: &mut Cursor<&[u8]>, channels: usize, bps: usize) -> (Frame, usize) {
    let mut samples : Vec<i32> = Vec::with_capacity(channels);

    for _ in 0..channels {
      match cursor.read_int::<LittleEndian>(bps) {
        Ok(sample) => samples.push(sample as i32),
        Err(e)     => { panic!("{:?}", e); }
      }
    }

    (samples, cursor.position() as usize)
  }

  fn next_float(cursor: &mut Cursor<&[u8]>, channels: usize, bps: usize) -> (Frame, usize) {
    if bps != 4 {
      panic!("Can't handle the specified bytes per sample");
    }

    let mut samples : Vec<i32> = Vec::with_capacity(channels);

    for _ in 0..channels {
      match cursor.read_f32::<LittleEndian>() {
        Ok(sample) => {
          let scaled = (sample * 2147483647.0) as i32;
          samples.push(scaled);
        },
        Err(e)     => { panic!("{:?}", e); }
      }
    }

    (samples, cursor.position() as usize)
  }

}

#[test]
fn test_info() {
  let file = match WaveFile::open("./fixtures/test-s24le.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e)
  };
  let info = file.info();

  assert_eq!(info.audio_format,    Format::PCM);
  assert_eq!(info.channels,        2);
  assert_eq!(info.sample_rate,     48000);
  assert_eq!(info.byte_rate,       288000);
  assert_eq!(info.block_align,     6);
  assert_eq!(info.bits_per_sample, 24);
  assert_eq!(info.total_frames,    501888);

  let file = match WaveFile::open("./fixtures/test-u8.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e)
  };
  let info = file.info();

  assert_eq!(info.audio_format,    Format::PCM);
  assert_eq!(info.channels,        2);
  assert_eq!(info.sample_rate,     48000);
  assert_eq!(info.byte_rate,       96000);
  assert_eq!(info.bits_per_sample, 8);
  assert_eq!(info.block_align,     2);
  assert_eq!(info.total_frames,    501888);
}

#[test]
fn test_iter() {
  let file = match WaveFile::open("./fixtures/test-s24le.wav") {
    Ok(f) => f,
    Err(e) => panic!("Error: {:?}", e)
  };

  let frames = file.iter().take(2).collect::<Vec<_>>();
  let expected = vec![
    [19581, 19581],
    [24337, 24337]
  ];

  for i in 0..expected.len() {
    assert_eq!(frames[i], expected[i]);
  }

  let frame = file.iter().last().unwrap();
  let expected = [244, 244];

  assert_eq!(frame, expected)
}


#[test]
fn test_float_extended() {
  let file = WaveFile::open("./fixtures/test-f32le.wav").unwrap();
  let info = file.info();

  assert_eq!(info.audio_format,  Format::Extended);
  assert_eq!(file.data_format(), Format::IEEEFloat);
  assert_eq!(file.len(),         501888);

  let frames = file.iter().take(2).collect::<Vec<_>>();
  // these are the same values as the 24-bit samples,
  // however we've scaled to 32-bit.
  let expected = vec![
    [5012736, 5012736],
    [6230272, 6230272]
  ];

  for i in 0..expected.len() {
    assert_eq!(frames[i], expected[i]);
  }

  assert_eq!(file.speakers().unwrap(),
             [SpeakerPosition::FrontLeft, SpeakerPosition::FrontRight]);
}
