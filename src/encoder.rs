use std::mem::MaybeUninit;

use ffmpeg::format::Pixel;
use ffmpeg::software::scaling::{flag::Flags};
use ffmpeg::util::frame;
use ffmpeg::format::{output_as, context::Output as OutputContext };
use ffmpeg::codec::{Id as CodecId};
use ffmpeg::codec::encoder::{find as find_codec};
use ffmpeg::encoder::{Video as VideoEncoder};
use ffmpeg::Rational;
use ffmpeg::Packet;
use ffmpeg::StreamMut;

use crate::request::Request;
use crate::processing::VideoInfo;

type ScalingContext = ffmpeg::software::scaling::Context;
type VideoFrame = frame::Video;

struct EncInit<'a, 'b, R: Into<Rational> + Copy + Clone> {
    request: &'a Request,
    video_info: &'a VideoInfo<R>,
    output: MaybeUninit<OutputContext>,
    scaler: MaybeUninit<ScalingContext>,
    encoder: MaybeUninit<VideoEncoder>,
    stream: MaybeUninit<StreamMut<'b>>,
    stream_index: usize,
}

impl<'a, 'b, R> EncInit<'a, 'b, R>
where R: Into<Rational> + Copy + Clone {
    unsafe fn assume_init(self) -> Encoder<'a, 'b, R> {
        Encoder {
            request: self.request,
            video_info: self.video_info,
            output: self.output.assume_init(),
            scaler: self.scaler.assume_init(),
            encoder: self.encoder.assume_init(),
            stream: self.stream.assume_init(),
            stream_index: self.stream_index,
        }
    }
}

pub struct Encoder<'a, 'b, R: Into<Rational> + Copy + Clone> {
    request: &'a Request,
    video_info: &'a VideoInfo<R>,
    output: OutputContext,
    scaler: ScalingContext,
    encoder: VideoEncoder,
    stream: StreamMut<'b>,
    stream_index: usize,
}

impl<'a, 'b, R> Encoder<'a, '_, R>
where R: Into<Rational> + Copy + Clone {
    const PIXEL_FORMAT: Pixel = Pixel::YUV420P;
    pub fn new(request: &'a Request, video_info: &'a VideoInfo<R>) -> Result<Self, ffmpeg::Error> {
        let mut this = EncInit {
            request,
            video_info,
            output: MaybeUninit::<OutputContext>::uninit(),
            scaler: MaybeUninit::<ScalingContext>::uninit(),
            encoder: MaybeUninit::<VideoEncoder>::uninit(),
            stream: MaybeUninit::<StreamMut<'_>>::uninit(),
            stream_index: 0,
        };

        let output = output_as(&request.output_path(), "webm").expect("Could not create the output file");
        unsafe { this.output.as_mut_ptr().write(output); }

        let scaler = ScalingContext::get(
            Pixel::RGB24,
            video_info.width,
            video_info.height,
            Self::PIXEL_FORMAT,
            video_info.width,
            video_info.height,
            Flags::BILINEAR).expect("Could not create the scaler");
        unsafe { this.scaler.as_mut_ptr().write(scaler); }

        let codec = find_codec(CodecId::VP9).ok_or(ffmpeg::Error::EncoderNotFound)?;

        let mut stream = unsafe { this.output.as_mut_ptr().as_mut() }.unwrap().add_stream(codec).expect("Could not add the video stream");
        let mut encoder = stream.codec().encoder().video().expect("Could not create the encoder");
        encoder.set_width(video_info.width);
        encoder.set_height(video_info.height);
        encoder.set_format(Self::PIXEL_FORMAT);
        encoder.set_gop(10);
        encoder.set_global_quality(32);
        encoder.set_frame_rate(Some(video_info.frame_rate));
        encoder.set_time_base(video_info.timebase);
        encoder.set_bit_rate(6 * 1024 * 1024);
        encoder.set_max_bit_rate(10 * 1024 * 1024);
        let encoder = encoder.open_as(codec).expect("Could not open the encoder");
        stream.set_parameters(&encoder);
        this.stream_index = stream.index();

        unsafe { this.encoder.as_mut_ptr().write(encoder); }
        unsafe { this.stream.as_mut_ptr().write(stream); }

        let mut this = unsafe { this.assume_init() };
        this.output.write_header()?;
        Ok(this)
    }

    pub fn encode_frame<'x>(&'x mut self, frame: &'x VideoFrame) -> Result<(), ffmpeg::Error> {
        let mut out_frame = VideoFrame::empty();
        self.scaler.run(frame, &mut out_frame)?;

        let mut out_packet = Packet::empty();
        if let Ok(true) = self.encoder.encode(&out_frame, &mut out_packet) {
            out_packet.set_stream(self.stream_index);
            out_packet.write_interleaved(&mut self.output)?;
        }

        let mut out_packet = Packet::empty();
        if let Ok(true) = self.encoder.flush(&mut out_packet) {
            out_packet.set_stream(self.stream_index);
            out_packet.write_interleaved(&mut self.output)?;
        }

        Ok(())
    }

    pub fn finish<'x>(&'x mut self) -> Result<(), ffmpeg::Error> {
        let mut out_packet = Packet::empty();
        if let Ok(true) = self.encoder.flush(&mut out_packet) {
            out_packet.set_stream(self.stream_index);
            out_packet.write_interleaved(&mut self.output)?;
        }
        
        self.output.write_trailer()?;
        Ok(())
    }
}
