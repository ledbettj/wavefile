const SPEAKER_FRONT_LEFT: isize = 1;
const SPEAKER_FRONT_RIGHT: isize = 1 << 1;
const SPEAKER_FRONT_CENTER: isize = 1 << 2;
const SPEAKER_LOW_FREQUENCY: isize = 1 << 3;
const SPEAKER_BACK_LEFT: isize = 1 << 4;
const SPEAKER_BACK_RIGHT: isize = 1 << 5;
const SPEAKER_FRONT_LEFT_OF_CENTER: isize = 1 << 6;
const SPEAKER_FRONT_RIGHT_OF_CENTER: isize = 1 << 7;
const SPEAKER_BACK_CENTER: isize = 1 << 8;
const SPEAKER_SIDE_LEFT: isize = 1 << 9;
const SPEAKER_SIDE_RIGHT: isize = 1 << 10;
const SPEAKER_TOP_CENTER: isize = 1 << 11;
const SPEAKER_TOP_FRONT_LEFT: isize = 1 << 12;
const SPEAKER_TOP_FRONT_CENTER: isize = 1 << 13;
const SPEAKER_TOP_FRONT_RIGHT: isize = 1 << 14;
const SPEAKER_TOP_BACK_LEFT: isize = 1 << 15;
const SPEAKER_TOP_BACK_CENTER: isize = 1 << 16;
const SPEAKER_TOP_BACK_RIGHT: isize = 1 << 17;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SpeakerPosition {
  FrontLeft = SPEAKER_FRONT_LEFT,
  FrontRight = SPEAKER_FRONT_RIGHT,
  FrontCenter = SPEAKER_FRONT_CENTER,
  LowFrequency = SPEAKER_LOW_FREQUENCY,
  BackLeft = SPEAKER_BACK_LEFT,
  BackRight = SPEAKER_BACK_RIGHT,
  FrontLeftOfCenter = SPEAKER_FRONT_LEFT_OF_CENTER,
  FrontRightOfCenter = SPEAKER_FRONT_RIGHT_OF_CENTER,
  BackCenter = SPEAKER_BACK_CENTER,
  SideLeft = SPEAKER_SIDE_LEFT,
  SideRight = SPEAKER_SIDE_RIGHT,
  TopCenter = SPEAKER_TOP_CENTER,
  TopFrontLeft = SPEAKER_TOP_FRONT_LEFT,
  TopFrontCenter = SPEAKER_TOP_FRONT_CENTER,
  TopFrontRight = SPEAKER_TOP_FRONT_RIGHT,
  TopBackLeft = SPEAKER_TOP_BACK_LEFT,
  TopBackCenter = SPEAKER_TOP_BACK_CENTER,
  TopBackRight = SPEAKER_TOP_BACK_RIGHT,
}

impl SpeakerPosition {
  pub fn decode(bits: isize) -> Vec<SpeakerPosition> {
    let mut speakers = Vec::with_capacity(8);
    let mut i = SPEAKER_FRONT_LEFT;

    while i <= SPEAKER_TOP_BACK_RIGHT {
      if bits & i != 0 {
        speakers.push(match i {
          SPEAKER_FRONT_LEFT => SpeakerPosition::FrontLeft,
          SPEAKER_FRONT_RIGHT => SpeakerPosition::FrontRight,
          SPEAKER_FRONT_CENTER => SpeakerPosition::FrontCenter,
          SPEAKER_LOW_FREQUENCY => SpeakerPosition::LowFrequency,
          SPEAKER_BACK_LEFT => SpeakerPosition::BackLeft,
          SPEAKER_BACK_RIGHT => SpeakerPosition::BackRight,
          SPEAKER_FRONT_LEFT_OF_CENTER => SpeakerPosition::FrontLeftOfCenter,
          SPEAKER_FRONT_RIGHT_OF_CENTER => SpeakerPosition::FrontRightOfCenter,
          SPEAKER_BACK_CENTER => SpeakerPosition::BackCenter,
          SPEAKER_SIDE_LEFT => SpeakerPosition::SideLeft,
          SPEAKER_SIDE_RIGHT => SpeakerPosition::SideRight,
          SPEAKER_TOP_CENTER => SpeakerPosition::TopCenter,
          SPEAKER_TOP_FRONT_LEFT => SpeakerPosition::TopFrontLeft,
          SPEAKER_TOP_FRONT_CENTER => SpeakerPosition::TopFrontCenter,
          SPEAKER_TOP_FRONT_RIGHT => SpeakerPosition::TopFrontRight,
          SPEAKER_TOP_BACK_LEFT => SpeakerPosition::TopBackLeft,
          SPEAKER_TOP_BACK_CENTER => SpeakerPosition::TopBackCenter,
          SPEAKER_TOP_BACK_RIGHT => SpeakerPosition::TopBackRight,
          _ => unreachable!(),
        });
      }
      i <<= 1;
    }
    speakers
  }
}
