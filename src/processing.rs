use std::iter::Enumerate;
use std::cell::{RefCell};

use ffmpeg::format::{Pixel, context::input::PacketIter};
use ffmpeg::media::Type;
use ffmpeg::decoder;
use ffmpeg::software::scaling::{flag::Flags};
use ffmpeg::util::frame;

use image;

use img_hash::{HasherConfig, Hasher, ImageHash};

use crate::request::Request;

type InputContext = ffmpeg::format::context::Input;
type ScalingContext = ffmpeg::software::scaling::Context;
type VideoDecoder = decoder::Video;
type VideoFrame = frame::Video;

pub struct TimelapseContext<'a> {
    request: &'a Request,

    packet_iter: Enumerate<PacketIter<'a>>,
    decoder: VideoDecoder,
    scaler: ScalingContext,

    video_stream_id: usize,
    frame_n: usize,

    hasher: Hasher,
    last_hash: RefCell<Option<ImageHash>>,
}

impl<'a> TimelapseContext<'a> {
    pub fn new(ictx: &'a mut InputContext, request: &'a Request) -> Result<Self, ffmpeg::Error> {
        //let mut ictx = input(&request.input_path())?;
        let stream = ictx.streams().best(Type::Video).ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_id = stream.index();
        let decoder = stream.codec().decoder().video()?;
        let scaler = ScalingContext::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR
        )?;

        let hasher = HasherConfig::new().to_hasher();
        let packet_iter = ictx.packets().enumerate();

        Ok(Self {
            request,

            decoder,
            scaler,
            video_stream_id,
            hasher,

            frame_n: 0,
            packet_iter,
            last_hash: RefCell::new(None),
        })
    }

    pub fn next_frame<'b>(&'b mut self) -> Result<VideoFrame, ffmpeg::Error> {
        let mut window = self.next_window()?;

        if self.last_hash.borrow().is_none() {
            let frame = window.remove(0);
            let hash = self.hash_frame(&frame);
            self.last_hash.replace(Some(hash));
            return Ok(frame);
        }

        let last_hash = self.last_hash.borrow().clone().unwrap();
        if self.request.verbose { println!("last hash: {}", last_hash.to_base64()); }
        let mut last_distance = u32::max_value();
        let mut last_frame: Option<VideoFrame> = None;
        let mut current_hash: Option<ImageHash> = None;
        // TODO: parallelize this loop with Rayon
        for frame in window {
            let hash = self.hash_frame(&frame);

            match last_frame {
                Some(_) => if last_hash.dist(&hash) < last_distance {
                    last_frame = Some(frame);
                    last_distance = last_hash.dist(&hash);
                    if self.request.verbose { println!("    new hash: {}; distance: {}", hash.to_base64(), last_distance); }
                    current_hash = Some(hash);
                },
                None => {
                    last_frame = Some(frame);
                    last_distance = last_hash.dist(&hash);
                    if self.request.verbose { println!("    new hash: {}; distance: {}", hash.to_base64(), last_distance); }
                    current_hash = Some(hash);
                }
            }
        }

        self.last_hash.replace(Some(current_hash.unwrap()));

        match last_frame {
            Some(frame) => {
                Ok(frame)
            },
            None => Err(ffmpeg::Error::Eof)
        }
    }

    fn next_window<'b>(&'b mut self) -> Result<Vec<VideoFrame>, ffmpeg::Error> {
        let mut window = Vec::<VideoFrame>::new();
        let mut skip_count = self.request.frame_skip;

        while window.len() < self.request.window_size as usize {
            match self.packet_iter.next() {
                Some((_, (s, packet))) => {
                    if s.index() != self.video_stream_id {
                        continue;
                    }

                    let mut frame = VideoFrame::empty();
                    self.decoder.decode(&packet, &mut frame)?;
                    if self.request.key_frames_only && !frame.is_key() {
                        continue;
                    }

                    if skip_count > 0 {
                        skip_count -= 1;
                        continue;
                    }

                    let mut rgb_frame = VideoFrame::empty();
                    self.scaler.run(&frame, &mut rgb_frame)?;

                    window.push(rgb_frame);
                },
                None => break
            }
        }

        Ok(window)
    }

    fn hash_frame<'b>(&'b self, frame: &VideoFrame) -> ImageHash {
        let data = frame.data(0).to_vec();

        let buffer = image::FlatSamples {
            samples: data,
            layout: image::flat::SampleLayout::row_major_packed(3, frame.width(), frame.height()),
            color_hint: Some(image::ColorType::Rgb8),
        };

        let img_buffer = buffer.try_into_buffer::<image::Rgb<u8>>().unwrap();
        self.hasher.hash_image(&img_buffer)
    }
}
