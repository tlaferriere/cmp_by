use cmp_by_derive::CmpBy;
use core::cmp::Ordering;

#[test]
fn test_cmp_by() {
    #[derive(Ord, PartialOrd, Eq, PartialEq)]
    struct Midi {
        global_time: usize,
        note: Note,
    }

    #[derive(CmpBy, Debug)]
    #[cmp_by(channel(), pitch(), _fields)]
    enum Note {
        NoteOn { pitch: u8, channel: u8 },
        NoteOff { pitch: u8, channel: u8 },
        CC,
        Unsupported { raw_data: Vec<u8>, channel: u8 },
    }

    impl Note {
        fn channel(&self) -> Option<&u8> {
            match self {
                Note::CC => None,
                Note::NoteOn { channel, .. }
                | Note::NoteOff { channel, .. }
                | Note::Unsupported { channel, .. } => Some(channel),
            }
        }

        fn pitch(&self) -> Option<&u8> {
            match self {
                Note::NoteOn { pitch, .. } | Note::NoteOff { pitch, .. } => Some(pitch),
                _ => None,
            }
        }
    }

    assert_eq!(
        Midi {
            global_time: 0,
            note: Note::NoteOn {
                pitch: 0,
                channel: 0,
            }
        }
        .cmp(&Midi {
            global_time: 0,
            note: Note::NoteOn {
                pitch: 0,
                channel: 0,
            }
        }),
        Ordering::Equal
    );
    assert_eq!(
        Midi {
            global_time: 0,
            note: Note::NoteOn {
                pitch: 2,
                channel: 2,
            }
        }
        .cmp(&Midi {
            global_time: 2,
            note: Note::NoteOff {
                pitch: 0,
                channel: 0,
            }
        }),
        Ordering::Less
    );
    assert_eq!(
        Midi {
            global_time: 0,
            note: Note::NoteOn {
                pitch: 2,
                channel: 0,
            }
        }
        .cmp(&Midi {
            global_time: 0,
            note: Note::NoteOff {
                pitch: 0,
                channel: 2,
            }
        }),
        Ordering::Less
    );
    assert_eq!(
        Midi {
            global_time: 0,
            note: Note::NoteOn {
                pitch: 0,
                channel: 0,
            }
        }
        .cmp(&Midi {
            global_time: 0,
            note: Note::NoteOff {
                pitch: 0,
                channel: 2,
            }
        }),
        Ordering::Less
    );
    assert_eq!(
        Midi {
            global_time: 0,
            note: Note::NoteOn {
                pitch: 0,
                channel: 0,
            }
        }
        .cmp(&Midi {
            global_time: 0,
            note: Note::NoteOff {
                pitch: 0,
                channel: 0,
            }
        }),
        Ordering::Less
    );
}
