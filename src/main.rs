use opencv::{
    core,
    highgui,
    prelude::*,
    videoio,
};

fn test_cv(input_file: &str) -> opencv::Result<()> {
    let window = "test";
    let mut video = videoio::VideoCapture::from_file(input_file, videoio::CAP_ANY)?;

    highgui::named_window(window, 1)?;

    loop {
        let mut frame = core::Mat::default()?;
        video.read(&mut frame)?;

        if frame.size()?.width > 0 {
            highgui::imshow(window, &mut frame)?;
        }

        let key = highgui::wait_key(10)?;
        if key > 0 && key != 255 {
            break;
        }
    }

    Ok(())
}

fn main() {
    test_cv("C:\\Users\\ftwie\\Documents\\Projects\\timelapse\\video.mp4").unwrap();
}
