use twp::builders::FrameBuilderError::*;
use twp::builders::*;

#[test]
fn bad_offset() {
    let mut frames = [0; 32];

    assert_eq!(
        set_stream_id(&mut frames, 1, 0x01, true),
        Err(InvalidOffset(1))
    );

    assert_eq!(
        set_stream_id(&mut frames, 32, 0x01, true),
        Err(InvalidOffset(32))
    );

    assert_eq!(
        set_stream_id(&mut frames, 17, 0x01, true),
        Err(InvalidOffset(17))
    );

    assert_eq!(
        set_stream_data(&mut frames, 32, 0x01),
        Err(InvalidOffset(32))
    );
}

#[test]
fn bad_offset_aux() {
    let mut frames = [0; 32];

    assert_eq!(
        set_stream_id(&mut frames, 15, 0x01, true),
        Err(InvalidOffset(15))
    );

    assert_eq!(
        set_stream_data(&mut frames, 31, 0x01),
        Err(InvalidOffset(31))
    );
}

#[test]
fn bad_stream_id() {
    let mut frames = [0; 32];

    assert_eq!(
        set_stream_id(&mut frames, 0, 0x7F, true),
        Err(InvalidStreamId(0, 0x7F))
    );
}

#[test]
fn invalid_delayed_id() {
    let mut frames = [0; 32];

    assert_eq!(
        set_stream_id(&mut frames, 14, 1, false),
        Err(InvalidDelayedId(14, 1))
    );
}

#[test]
fn immediate_id() {
    let mut frames = [0; 32];
    let mut exp = [0; 32];
    assert_eq!(set_stream_id(&mut frames, 2, 1, true), Ok(()));
    assert_eq!(set_stream_id(&mut frames, 18, 1, true), Ok(()));

    exp[2] = 0x03;
    exp[18] = 0x03;
    assert_eq!(frames, exp);
}

#[test]
fn delayed_id() {
    let mut frames = [0; 32];
    let mut exp = [0; 32];
    assert_eq!(set_stream_id(&mut frames, 2, 1, false), Ok(()));
    assert_eq!(set_stream_id(&mut frames, 18, 1, false), Ok(()));

    exp[2] = 0x03;
    exp[15] = 0x02;
    exp[18] = 0x03;
    exp[31] = 0x02;
    assert_eq!(frames, exp);
}

#[test]
fn basic_data() {
    let mut frames = [0; 32];
    assert_eq!(set_stream_data(&mut frames, 0, 3), Ok(()));
    assert_eq!(set_stream_data(&mut frames, 1, 3), Ok(()));
    assert_eq!(set_stream_data(&mut frames, 16, 3), Ok(()));
    assert_eq!(set_stream_data(&mut frames, 17, 3), Ok(()));

    let mut exp = [0; 32];
    exp[0] = 0x02;
    exp[1] = 0x03;
    exp[15] = 0x01;
    exp[16] = 0x02;
    exp[17] = 0x03;
    exp[31] = 0x01;
    assert_eq!(frames, exp);
}

#[test]
fn frame_builder_basic_data() {
    let mut fb = FrameBuilder::new(0);
    assert_eq!(fb.set_data(3), Ok(()));
    assert_eq!(fb.set_data(3), Ok(()));

    let mut exp = [0; 16];
    exp[0] = 0x02;
    exp[1] = 0x03;
    exp[15] = 0x01;
    assert_eq!(fb.build(), exp);
}

#[test]
fn frame_builder_basic_id() {
    let mut fb = FrameBuilder::new(0);
    assert_eq!(fb.set_id(1), Ok(()));
    assert_eq!(fb.set_data(1), Ok(()));
    assert_eq!(fb.set_data(2), Ok(()));
    assert_eq!(fb.set_id(2), Ok(()));

    let mut exp = [0; 16];
    exp[0] = 0x03;
    exp[1] = 0x01;
    exp[2] = 0x05;
    exp[3] = 0x02;
    exp[15] = 0x02;
    assert_eq!(fb.build(), exp);
}

#[test]
fn penultimate_id_change() {
    let frames = FrameBuilder::new(0)
        .data_span(14, 2)
        .id(2)
        .data_span(15, 2)
        .build();

    let mut exp = [2; 32];
    exp[14] = 0x05;
    exp[15] = 0;
    exp[31] = 0;
    assert_eq!(frames, exp);
}

#[test]
#[should_panic]
fn missing_data() {
    let _frames = FrameBuilder::new(0).data_span(1, 2).id(2).id(3).build();
}

#[test]
#[should_panic]
fn missing_data2() {
    let _frames = FrameBuilder::new(0).data(1).id(2).id(3).build();
}

#[test]
fn fancy_span() {
    let mut i = 0;
    let frames = FrameBuilder::new(0)
        .data_span_with(15, || {
            i += 1;
            i
        })
        .build();
    let mut exp = [0; 16];
    exp[0] = 0;
    exp[1] = 2;
    exp[2] = 2;
    exp[3] = 4;
    exp[4] = 4;
    exp[5] = 6;
    exp[6] = 6;
    exp[7] = 8;
    exp[8] = 8;
    exp[9] = 10;
    exp[10] = 10;
    exp[11] = 12;
    exp[12] = 12;
    exp[13] = 14;
    exp[14] = 14;
    exp[15] = 0xFF;

    assert_eq!(frames, exp);
}
