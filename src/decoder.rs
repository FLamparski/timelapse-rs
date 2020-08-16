use ffmpeg::codec::packet::flag::Flags as PacketFlags;
use ffmpeg::format::{Pixel, context::input::{Input as InputContext, PacketIter, dump as dump_format}};
use ffmpeg::media::Type;
use ffmpeg::decoder::{Video as VideoDecoder};
use ffmpeg::software::scaling::{flag::Flags as ScalingFlags, Context as ScalingContext};
use ffmpeg::util::frame::{Video as VideoFrame};
use ffmpeg::Rational;

use crate::request::{Request, ComparisonMode};

pub struct Decoder<'a> {
    request: &'a Request,

    packet_iter: PacketIter<'a>,
    decoder: VideoDecoder,
    scaler: ScalingContext,

    video_stream_id: usize,
    num_frames: i64,
}

impl<'a> Decoder<'a> {
    pub fn new(request: &'a Request, ictx: &'a mut InputContext) -> Result<Self, ffmpeg::Error> {
        if request.verbose > 0 { dump_format(&ictx, 0, request.input_path().to_str()); }

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
            ScalingFlags::BILINEAR
        )?;

        Ok(Self {
            request,

            decoder,
            scaler,
            video_stream_id,
            num_frames,

            packet_iter: ictx.packets(),
        })
    }

    pub fn get_info(&self) -> VideoInfo<Rational> {
        VideoInfo {
            width: self.decoder.width(),
            height: self.decoder.height(),
            frame_rate: self.decoder.frame_rate().unwrap(),
            timebase: self.decoder.time_base(),
            total_frames: self.num_frames,
            decoded_pixel_format: output_pixel_format(self.request.comparison_mode),
        }
    }


    pub fn next_window<'x>(&'x mut self) -> Result<Vec<VideoFrame>, ffmpeg::Error> {
        let mut window = Vec::<VideoFrame>::new();

        while window.len() < self.request.window_size as usize {
            match self.next_frame() {
                Ok(frame) => window.push(frame),
                Err(ffmpeg::Error::Eof) => break,
                Err(e) => return Err(e)
            }
        }

        if window.is_empty() {
            Err(ffmpeg::Error::Eof)
        } else {
            Ok(window)
        }
    }

    pub fn next_frame<'x>(&'x mut self) -> Result<VideoFrame, ffmpeg::Error> {
        let mut skip_count = self.request.frame_skip;

        loop {
            match self.packet_iter.next() {
                Some((s, packet)) => {
                    if s.index() != self.video_stream_id {
                        if self.request.verbose > 2 { println!("decoder::next_frame: skip packet {} (stream {} != video stream {})", packet.position(), s.index(), self.video_stream_id); }
                        continue;
                    }

                    let is_key = packet.flags().intersects(PacketFlags::KEY);
                    if self.request.key_frames_only && !is_key {
                        if self.request.verbose > 2 { println!("decoder::next_frame: skip packet {} (not a key frame but --key-frames-only is set)", packet.position()); }
                        continue;
                    }

                    if skip_count > 0 {
                        if self.request.verbose > 2 { println!("decoder::next_frame: skip packet {} (skip count = {})", packet.position(), skip_count); }
                        skip_count -= 1;
                        continue;
                    }

                    let mut frame = VideoFrame::empty();
                    self.decoder.decode(&packet, &mut frame)?;

                    if unsafe { frame.is_empty() } {
                        if self.request.verbose > 2 { println!("decoder::next_frame: skip empty frame at {}", packet.position()); }
                        continue;
                    }

                    let mut scaled_frame = VideoFrame::empty();
                    self.scaler.run(&frame, &mut scaled_frame)?;

                    return Ok(scaled_frame);
                },
                None => return Err(ffmpeg::Error::Eof),
            }
        }
    }
}

fn output_pixel_format(comparison_mode: ComparisonMode) -> Pixel {
    match comparison_mode {
        ComparisonMode::Blockhash | ComparisonMode::GradientHash | ComparisonMode::MeanHash => Pixel::RGB24,
        ComparisonMode::MSE | ComparisonMode::SSIM | ComparisonMode::Noop => Pixel::YUV420P
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VideoInfo<R: Into<Rational> + Copy + Clone> {
    pub width: u32,
    pub height: u32,
    pub frame_rate: R,
    pub timebase: R,
    pub total_frames: i64,
    pub decoded_pixel_format: Pixel,
}
