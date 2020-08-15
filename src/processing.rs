use std::iter::Enumerate;
use std::cell::{RefCell};

use rayon::prelude::*;

use ffmpeg::format::{Pixel, context::input::PacketIter};
use ffmpeg::media::Type;
use ffmpeg::decoder;
use ffmpeg::software::scaling::{flag::Flags};
use ffmpeg::util::frame;
use ffmpeg::Rational;

use image;

use img_hash::{HasherConfig, HashAlg, ImageHash};

use crate::request::Request;

type InputContext = ffmpeg::format::context::Input;
type ScalingContext = ffmpeg::software::scaling::Context;
type VideoDecoder = decoder::Video;
type VideoFrame = frame::Video;
type PacketFlags = ffmpeg_next::codec::packet::flag::Flags;

pub struct TimelapseContext<'a> {
    request: &'a Request,

    packet_iter: Enumerate<PacketIter<'a>>,
    decoder: VideoDecoder,
    scaler: ScalingContext,

    video_stream_id: usize,
    num_frames: i64,

    last_hash: RefCell<Option<ImageHash>>,
}

impl<'a> TimelapseContext<'a> {
    pub fn new(ictx: &'a mut InputContext, request: &'a Request) -> Result<Self, ffmpeg::Error> {
        if request.verbose > 1 { println!("TimelapseContext::new found {} streams in file", ictx.streams().count()); }

        let stream = ictx.streams().best(Type::Video).ok_or(ffmpeg::Error::StreamNotFound)?;
        if request.verbose > 2 { println!("TimelapseContext::new found video stream at #{}", stream.index()); }

        let num_frames = stream.frames();
        if request.verbose > 2 { println!("TimelapseContext::new stream appears to have {} frames", num_frames); }

        let video_stream_id = stream.index();
        let decoder = stream.codec().decoder().video()?;
        if request.verbose > 2 { println!("TimelapseContext::new codec appears to be {:?}", decoder.id()); }

        let scaler = ScalingContext::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR
        )?;

        let packet_iter = ictx.packets().enumerate();

        Ok(Self {
            request,

            decoder,
            scaler,
            video_stream_id,
            num_frames,

            packet_iter,
            last_hash: RefCell::new(None),
        })
    }

    pub fn get_info(&self) -> VideoInfo<Rational> {
        VideoInfo {
            width: self.decoder.width(),
            height: self.decoder.height(),
            frame_rate: self.decoder.frame_rate().unwrap(),
            timebase: self.decoder.time_base(),
            total_frames: self.num_frames,
        }
    }

    pub fn next_frame<'b>(&'b mut self) -> Result<VideoFrame, ffmpeg::Error> {
        let mut window = self.next_window()?;
        let request = self.request;

        if self.last_hash.borrow().is_none() {
            let frame = window.remove(0);
            let hash = hash_frame(&frame);
            self.last_hash.replace(Some(hash));
            return Ok(frame);
        }

        let last_hash = self.last_hash.borrow().clone().unwrap();
        if request.verbose > 2 { println!("last hash: {}", last_hash.to_base64()); }

        let hashing_result = window.into_par_iter().map(|frame| {
            let hash = hash_frame(&frame);
            let dist = last_hash.dist(&hash);
            if request.verbose > 5 { println!("    candidate hash: {} (distance {})", hash.to_base64(), dist); }
            (frame, hash, dist)
        }).min_by_key(|&(_, _, dist)| dist);

        if let Some((frame, hash, dist)) = hashing_result {
            if request.verbose > 2 { println!("    selected hash: {} (distance {})", hash.to_base64(), dist); }
            self.last_hash.replace(Some(hash));
            Ok(frame)
        } else {
            if request.verbose > 0 { println!("end of file reached"); }
            Err(ffmpeg::Error::Eof)
        }
    }

    fn next_window<'b>(&'b mut self) -> Result<Vec<VideoFrame>, ffmpeg::Error> {
        let mut window = Vec::<VideoFrame>::new();
        let mut skip_count = self.request.frame_skip;

        while window.len() < self.request.window_size as usize {
            match self.packet_iter.next() {
                Some((_, (s, packet))) => {
                    if self.request.verbose > 3 { println!("next_window: read packet {}", packet.position()); }
                    if s.index() != self.video_stream_id {
                        if self.request.verbose > 2 { println!("next_window: skip packet {} (stream {} != video stream {})", packet.position(), s.index(), self.video_stream_id); }
                        continue;
                    }

                    let is_key = packet.flags().intersects(PacketFlags::KEY);
                    if self.request.key_frames_only && !is_key {
                        if self.request.verbose > 2 { println!("next_window: skip packet {} (not a key frame but --key-frames-only is set)", packet.position()); }
                        continue;
                    }

                    if skip_count > 0 {
                        if self.request.verbose > 2 { println!("next_window: skip packet {} (skip count = {})", packet.position(), skip_count); }
                        skip_count -= 1;
                        continue;
                    }

                    // It would be good to parallelise decoding the frames, however with just
                    // Rayon this isn't possible as the scaling context is not thread safe.
                    // Perhaps having multiple thread-local scaling contexts is a start, however
                    // this is not the main bottleneck.
                    let mut frame = VideoFrame::empty();
                    self.decoder.decode(&packet, &mut frame)?;

                    if unsafe { frame.is_empty() } {
                        if self.request.verbose > 2 { println!("next_window: skip empty frame at {}", packet.position()); }
                        continue;
                    }

                    let mut rgb_frame = VideoFrame::empty();
                    let scaler_result = self.scaler.run(&frame, &mut rgb_frame);

                    if let Err(e) = scaler_result {
                        if self.request.verbose > 1 {
                            println!("next_window: packet at {} could not be scaled: {}", packet.position(), e);
                            println!("next_window: frame reports width {} and height {}, is corrupt? {}, is empty? {}", frame.width(), frame.height(), frame.is_corrupt(), unsafe { frame.is_empty() });
                        }
                        return Err(e);
                    }

                    window.push(rgb_frame);

                    skip_count = self.request.frame_skip;
                },
                None => {
                    if self.request.verbose > 2 { println!("next_window: stream finished"); }
                    break;
                }
            }
        }

        Ok(window)
    }
}

fn hash_frame<'b>(frame: &VideoFrame) -> ImageHash {
    // Blockhash is fast but might not work in all cases
    let hasher = HasherConfig::new().hash_alg(HashAlg::Blockhash).to_hasher();
    let data = frame.data(0).to_vec();

    let buffer = image::FlatSamples {
        samples: data,
        layout: image::flat::SampleLayout::row_major_packed(3, frame.width(), frame.height()),
        color_hint: Some(image::ColorType::Rgb8),
    };

    let img_buffer = buffer.try_into_buffer::<image::Rgb<u8>>().unwrap();
    hasher.hash_image(&img_buffer)
}

#[derive(Debug, Copy, Clone)]
pub struct VideoInfo<R: Into<Rational> + Copy + Clone> {
    pub width: u32,
    pub height: u32,
    pub frame_rate: R,
    pub timebase: R,
    pub total_frames: i64,
}
