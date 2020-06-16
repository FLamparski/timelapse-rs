extern crate ffmpeg_next as ffmpeg;

use std::path::Path;
use std::boxed::Box;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use image;
use img_hash::{HasherConfig, Hasher, ImageHash};

fn hash_frame(hasher: &Hasher, rgb_frame: &Video) -> ImageHash {
    let data = rgb_frame.data(0).to_vec();
    let buffer = image::FlatSamples {
        samples: data,
        layout: image::flat::SampleLayout::row_major_packed(3, rgb_frame.width(), rgb_frame.height()),
        color_hint: Some(image::ColorType::Rgb8),
    };

    let img_buffer = buffer.try_into_buffer::<image::Rgb<u8>>().unwrap();

    return hasher.hash_image(&img_buffer);
}

fn ff_test<'a>(filename: &str) -> Result<(), ffmpeg::Error> {
    let path = Path::new(filename);
    let mut ictx = input(&path)?;

    let stream = ictx.streams().best(Type::Video).ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_index = stream.index();
    let mut decoder = stream.codec().decoder().video()?;
    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR
    )?;

    let mut frame_n = 0;
    let mut frames: Vec<Box<Video>> = Vec::new();
    for (i, (s, p)) in ictx.packets().enumerate() {
        if s.index() != video_index {
            continue;
        }

        if frame_n >= 2 {
            let hasher = HasherConfig::new().to_hasher();
            let hash1 = hash_frame(&hasher, frames.get(0).unwrap());
            let hash2 = hash_frame(&hasher, frames.get(1).unwrap());
            println!("Hash 1:   {}", hash1.to_base64());
            println!("Hash 2:   {}", hash2.to_base64());
            println!("Distance: {}", hash1.dist(&hash2));

            return Ok(());
        }

        let mut frame = Video::empty();
        match decoder.decode(&p, &mut frame) {
            Ok(_) => {
                if !frame.is_key() {
                    continue;
                }
                frame_n += 1;
                let mut rgb_frame = Video::empty();
                scaler.run(&frame, &mut rgb_frame).unwrap();

                frames.push(Box::new(rgb_frame));
            }
            Err(e) => println!("Error reading frame {}: {}", i, e)
        }
    }

    Ok(())
}

fn main() {
    ffmpeg::init().unwrap();

    ff_test("C:\\Users\\ftwie\\Documents\\Projects\\timelapse\\video.mp4").unwrap();
}
