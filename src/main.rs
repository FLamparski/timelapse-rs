extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input};
use ffmpeg::ffi::{av_log_set_level, AV_LOG_ERROR, AV_LOG_INFO, AV_LOG_DEBUG};
use structopt::StructOpt;

mod request;
mod processing;
mod encoder;
use crate::request::Request;
use crate::processing::TimelapseContext;
use crate::encoder::Encoder;

fn main() {
    let request = Request::from_args();
    init_ffmpeg(&request);

    let mut ictx = input(&request.input_path()).unwrap();
    let mut context = TimelapseContext::new(&mut ictx, &request).unwrap();

    let vid_info = context.get_info();
    
    if request.verbose > 1 { println!("{:#?}", request); }
    let num_output_frames = vid_info.total_frames / request.window_size as i64;
    println!("Will process {} input frames into {} output frames", vid_info.total_frames, num_output_frames);

    let mut encoder = Encoder::new(&request, &vid_info).unwrap();

    let mut i = 0u32;
    while let Ok(frame) = context.next_frame() {
        let percentage = (i as f64 / num_output_frames as f64) * 100.0;
        if i % 5 == 0 { println!("{}/{} ({:.1}% done)", i, num_output_frames, percentage); }
        encoder.encode_frame(&frame).unwrap();
        i += 1;
    }

    encoder.finish().unwrap();

    println!("All done - check {}!", request.output_path().display());
}

fn init_ffmpeg(request: &Request) {
    let log_level = match request.verbose {
        0 => AV_LOG_ERROR,
        1 => AV_LOG_INFO,
        _ => AV_LOG_DEBUG,
    };
    unsafe { av_log_set_level(log_level) };

    ffmpeg::init().unwrap();
}
