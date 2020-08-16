use std::cell::RefCell;

use ffmpeg::util::frame::Video as VideoFrame;

use rayon::prelude::*;

use image;

use img_hash::{HasherConfig, HashAlg, ImageHash};

use crate::request::{Request, ComparisonMode};

pub trait FrameSelector {
    fn pick_best(&mut self, window: Vec<VideoFrame>) -> Result<VideoFrame, FrameSelectionError>;
}

pub fn get_frame_selector<'a>(request: &'a Request) -> Box<dyn FrameSelector + 'a> {
    match request.comparison_mode {
        ComparisonMode::Noop => Box::new(NoopFrameSelector),
        ComparisonMode::Blockhash | ComparisonMode::GradientHash | ComparisonMode::MeanHash => Box::new(HashFrameSelector::new(request)),
        ComparisonMode::MSE => Box::new(MSEFrameSelector::new(request)),
        _ => panic!("Requested unsupported frame selector: {:?}", request.comparison_mode),
    }
}

struct MSEFrameSelector<'a> {
    request: &'a Request,
    last_frame: RefCell<Option<Vec<u8>>>,
}

impl<'a> FrameSelector for MSEFrameSelector<'a> {
    fn pick_best(&mut self, window: Vec<VideoFrame>) -> Result<VideoFrame, FrameSelectionError> {
        let mut window = window;
        if self.last_frame.borrow().is_none() {
            let frame = window.remove(0);
            self.last_frame.replace(Some(get_luma_data(&frame)));
            return Ok(frame);
        }

        let result = {
            let last_frame = self.last_frame.borrow();
            let previous_luma = last_frame.as_ref().unwrap();
            window.into_par_iter().map(|frame| {
                let luma = get_luma_data(&frame);
                let err = mse(&luma, previous_luma);
                (frame, luma, err)
            }).min_by(|(_, _, err1), (_, _, err2)| err1.partial_cmp(err2).unwrap_or(std::cmp::Ordering::Equal))
        };

        if let Some((frame, next_luma, err)) = result {
            if self.request.verbose > 2 { println!("mse = {}", err); }
            self.last_frame.replace(Some(next_luma));
            Ok(frame)
        } else {
            Err(FrameSelectionError::EmptyInput)
        }
    }
}

fn get_luma_data(frame: &VideoFrame) -> Vec<u8> {
    let mut luma_data = Vec::<u8>::new();
    for i in 0..(frame.data(0).len() / 3) {
        luma_data.push(frame.data(0)[i * 3]);
    }
    luma_data
}

fn mse(vec1: &Vec<u8>, vec2: &Vec<u8>) -> f64 {
    let sum: u32 = vec1.iter().zip(vec2.iter()).map(|(a, b)| {
        u32::from((i16::from(*a) - i16::from(*b)).saturating_pow(2) as u16)
    }).fold(0u32, |acc, x| acc.saturating_add(x));
    f64::from(sum) / f64::from(vec1.len() as u32)
}

impl<'a> MSEFrameSelector<'a> {
    fn new(request: &'a Request) -> MSEFrameSelector {
        MSEFrameSelector {
            request,
            last_frame: RefCell::new(None),
        }
    }
}

struct HashFrameSelector<'a> {
    request: &'a Request,
    last_hash: RefCell<Option<ImageHash>>,
}

impl<'a> HashFrameSelector<'a> {
    fn new(request: &'a Request) -> HashFrameSelector {
        HashFrameSelector {
            request,
            last_hash: RefCell::new(None),
        }
    }
}

fn hash_frame(frame: &VideoFrame, comparison_mode: ComparisonMode) -> ImageHash {
    // Blockhash is fast but might not work in all cases
    let hasher = HasherConfig::new().hash_alg(get_hash_alg(comparison_mode)).to_hasher();
    let data = frame.data(0).to_vec();

    let buffer = image::FlatSamples {
        samples: data,
        layout: image::flat::SampleLayout::row_major_packed(3, frame.width(), frame.height()),
        color_hint: Some(image::ColorType::Rgb8),
    };

    let img_buffer = buffer.try_into_buffer::<image::Rgb<u8>>().unwrap();
    hasher.hash_image(&img_buffer)
}

fn get_hash_alg(comparison_mode: ComparisonMode) -> HashAlg {
    match comparison_mode {
        ComparisonMode::Blockhash => HashAlg::Blockhash,
        ComparisonMode::GradientHash => HashAlg::DoubleGradient,
        ComparisonMode::MeanHash => HashAlg::Mean,
        _ => panic!("Invalid comparison mode given to HashFrameSelector: {:?}", comparison_mode)
    }
}

impl<'a> FrameSelector for HashFrameSelector<'a> {
    fn pick_best(&mut self, window: Vec<VideoFrame>) -> Result<VideoFrame, FrameSelectionError> {
        let mut window = window;
        if self.last_hash.borrow().is_none() {
            let frame = window.remove(0);
            let hash = hash_frame(&frame, self.request.comparison_mode);
            self.last_hash.replace(Some(hash));
            return Ok(frame);
        }

        let last_hash = self.last_hash.borrow().clone().unwrap();
        if self.request.verbose > 2 { println!("last hash: {}", last_hash.to_base64()); }

        let verbose = self.request.verbose;
        let comparison_mode = self.request.comparison_mode;
        let hashing_result = window.into_par_iter().map(|frame| {
            let hash = hash_frame(&frame, comparison_mode);
            let dist = last_hash.dist(&hash);
            if verbose > 5 { println!("    candidate hash: {} (distance {})", hash.to_base64(), dist); }
            (frame, hash, dist)
        }).min_by_key(|&(_, _, dist)| dist);

        if let Some((frame, hash, dist)) = hashing_result {
            if self.request.verbose > 2 { println!("    selected hash: {} (distance {})", hash.to_base64(), dist); }
            self.last_hash.replace(Some(hash));
            Ok(frame)
        } else {
            if self.request.verbose > 0 { println!("end of file reached"); }
            Err(FrameSelectionError::EmptyInput)
        }
    }
}

struct NoopFrameSelector;

impl FrameSelector for NoopFrameSelector {
    fn pick_best(&mut self, window: Vec<VideoFrame>) -> Result<VideoFrame, FrameSelectionError> {
        let mut window = window;
        if window.is_empty() {
            Err(FrameSelectionError::EmptyInput)
        } else {
            Ok(window.remove(0))
        }
    }
}

#[derive(Debug)]
pub enum FrameSelectionError {
    EmptyInput,
}
