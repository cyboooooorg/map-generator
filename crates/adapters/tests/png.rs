use adapters::{Ramp, encode_png};
use world::Raster;

#[test]
fn grey_ramp_maps_ends_and_middle() {
    let ramp = Ramp::grey(0.0, 1.0);
    assert_eq!(ramp.colour(0.0), [0, 0, 0]);
    assert_eq!(ramp.colour(1.0), [255, 255, 255]);
    assert_eq!(ramp.colour(0.5), [128, 128, 128]);
    assert_eq!(ramp.colour(-3.0), [0, 0, 0]);
    assert_eq!(ramp.colour(7.0), [255, 255, 255]);
    assert_eq!(ramp.colour(f32::NAN), [0, 0, 0]);
}

#[test]
fn a_png_decodes_to_its_size_and_colours() {
    let raster = Raster {
        width: 4,
        height: 2,
        values: vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.0, 0.0, 0.0],
    };
    let bytes = encode_png(&raster, &Ramp::grey(0.0, 1.0));
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width, info.height), (4, 2));
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    assert_eq!(&buf[0..3], &[0, 0, 0]);
    assert_eq!(&buf[12..15], &[255, 255, 255]);
}

#[test]
fn write_png_stages_then_leaves_only_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out").join("map.png");
    let raster = Raster {
        width: 2,
        height: 1,
        values: vec![0.0, 1.0],
    };
    adapters::write_png(&path, &raster, &Ramp::grey(0.0, 1.0)).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        encode_png(&raster, &Ramp::grey(0.0, 1.0))
    );
    assert!(!path.with_extension("png.tmp").exists());
}
