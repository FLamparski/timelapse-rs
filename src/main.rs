extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::input;

mod request;
mod processing;
use crate::request::Request;
use crate::processing::TimelapseContext;

fn main() {
    ffmpeg::init().unwrap();

    // Initial request - this should be read from the CLI
    let mut request = Request::new();
    request.set_input_path("C:\\Users\\ftwie\\Documents\\Projects\\timelapse\\video.mp4")
           .set_output_path("C:\\Users\\ftwie\\Documents\\Projects\\timelapse\\lapse-rs\\")
           .set_frame_skip(30)
           .set_verbose(true);

    let mut ictx = input(&request.input_path()).unwrap();
    let mut context = TimelapseContext::new(&mut ictx, &request).unwrap();

    for _ in 0..5 {
        context.next_frame().unwrap();
    }
}
