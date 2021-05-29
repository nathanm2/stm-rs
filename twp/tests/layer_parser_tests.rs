use std::result;
use twp::*;

#[derive(Debug, PartialEq)]
enum TestLayer {
    Padding {
        offset: usize,
    },
    FrameSync {
        offset: usize,
    },
    Frame {
        frame: [u8; 16],
        offsets: [usize; 16],
    },
}

use TestLayer::*;

impl<'a> std::convert::From<Layer<'a>> for TestLayer {
    fn from(layer: Layer) -> TestLayer {
        match layer {
            Layer::Padding { offset } => Padding { offset },
            Layer::FrameSync { offset } => FrameSync { offset },
            Layer::Frame { frame, offsets } => Frame {
                frame: frame.clone(),
                offsets: offsets.clone(),
            },
        }
    }
}

fn capture_layer(
    lr: Result<Layer>,
    layers: &mut Vec<TestLayer>,
    mut errors: Option<&mut Vec<Error>>,
) -> Result<()> {
    match lr {
        Ok(p) => {
            layers.push(TestLayer::from(p));
            Ok(())
        }
        Err(error) => match &mut errors {
            Some(v) => {
                v.push(error);
                Ok(())
            }
            None => Err(error),
        },
    }
}

fn capture_layers(
    stream: &[u8],
    parser: &mut LayerParser,
    layers: &mut Vec<TestLayer>,
    mut errors: Option<&mut Vec<Error>>,
) -> Result<()> {
    let mut c = |lr| capture_layer(lr, layers, errors.as_deref_mut());
    parser.parse(stream, c)
}

#[test]
fn no_sync() -> result::Result<(), Error> {
    let mut parser = LayerParser::new(false, true, 0);
    let stream = [1; 32];
    let mut packets = Vec::new();

    capture_layers(&stream, &mut parser, &mut packets, None)?;
    assert_eq!(packets.len(), 0);

    Ok(())
}

#[test]
fn basic_frames() -> result::Result<(), Error> {
    let mut parser = LayerParser::new(false, true, 0);
    let mut stream = [1; 4 + 16 + 16 + 2 + 2 + 2];
    let mut packets = Vec::new();

    stream[0] = 0xFF;
    stream[1] = 0xFF;
    stream[2] = 0xFF;
    stream[3] = 0x7F;

    stream[20] = 0xFF;
    stream[21] = 0x7F;

    stream[24] = 0xFF;
    stream[25] = 0x7F;
    stream[26] = 0xFF;
    stream[27] = 0x7F;

    stream[29] = 0xFF; // Not padding (not 16-bit aligned).
    stream[30] = 0x7F;

    capture_layers(&stream, &mut parser, &mut packets, None)?;

    let mut offsets1 = [0; 16];
    let mut offsets2 = [0; 16];

    for i in 0..16 {
        offsets1[i] = i + 4;
        offsets2[i] = i + 20;
    }

    let expected = vec![
        FrameSync { offset: 0 },
        Frame {
            frame: [1; 16],
            offsets: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
        },
        Padding { offset: 20 },
        Padding { offset: 24 },
        Padding { offset: 26 },
        Frame {
            frame: [1, 1, 1, 0xFF, 0x7F, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            offsets: [
                22, 23, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41,
            ],
        },
    ];

    assert_eq!(packets, expected);

    Ok(())
}
