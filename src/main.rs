extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input};
use ffmpeg::ffi::{av_log_set_level, AV_LOG_ERROR};

mod request;
mod processing;
mod encoder;
use crate::request::Request;
use crate::processing::TimelapseContext;
use crate::encoder::Encoder;

fn main() {
    unsafe { av_log_set_level(AV_LOG_ERROR); }
    ffmpeg::init().unwrap();

    // Initial request - this should be read from the CLI
    let mut request = Request::new();
    request.set_input_path("C:\\Users\\ftwie\\Documents\\Projects\\timelapse\\video.mp4")
           .set_output_path("C:\\Users\\ftwie\\Documents\\Projects\\timelapse\\rust-lapse.webm")
           .set_frame_skip(30)
           .set_verbose(false);

    let mut ictx = input(&request.input_path()).unwrap();
    let mut context = TimelapseContext::new(&mut ictx, &request).unwrap();

    let vid_info = context.get_info();
    let mut encoder = Encoder::new(&request, &vid_info).unwrap();

    const N_FRAMES: u32 = 300;
    for n in 0..N_FRAMES {
        if n % 10 == 0 { println!("{}/{}", n + 1, N_FRAMES); }
        if let Ok(frame) = context.next_frame() {
            encoder.encode_frame(&frame).unwrap();
        } else {
            println!("Finished on frame {}", n);
            break
        }
    }

    encoder.finish().unwrap();
}
