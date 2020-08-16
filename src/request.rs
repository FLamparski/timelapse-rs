use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::string::ToString;
use std::str::FromStr;
use std::fmt::{self, Display};

use structopt::StructOpt;

/// Processes videos into timelapses by selectively picking one for every window-size frames from
/// the input. The frame is selected based on its similarity to the previous frame, in order to
/// not result in a jittery sped-up video but something that's hopefully much smoother. The primary
/// use case for this program are 3D printing timelapses taken from a webcam.
#[derive(StructOpt, Debug)]
#[structopt(name = "timelapse-rs")]
pub struct Request {
    /// Path to the input file
    #[structopt(name = "INPUT", parse(from_os_str))]
    input_path: PathBuf,

    /// Path to the output file (.webm)
    #[structopt(name = "OUTPUT", parse(from_os_str))]
    output_path: PathBuf,

    /// Number of input frames to pick each output frame from
    #[structopt(long, default_value = "25")]
    pub window_size: u32,

    /// Number of input frames to skip for every output frame (may be useful for timelapses
    /// made from realtime videos)
    #[structopt(long, default_value = "0")]
    pub frame_skip: u32,

    /// Only use "key" frames from the input, eg. frames that encode a full image rather than those
    /// that encode differences between images. The behaviour of this option depends on the encoding
    /// of the input video, and may be useful for timelapses made from realtime videos.
    #[structopt(long)]
    pub key_frames_only: bool,

    /// Verbose output (-v, -vv, -vvv etc) - show messages from the app itself and from ffmpeg
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,

    /// How to compare frames to determine similarity
    /// 
    /// Current options:
    /// 
    /// * `blockhash` (from `img_hash`) - fast but often has poor results
    /// * `gradienthash` (from `img_hash`) - slower, potentially better results
    /// * `meanhash` (from `img_hash`) - slower, potentially better results
    /// * `mse` - mean square error - slow, but should have good results
    /// * `ssim` - structured similarity index - slowest, but should have best results
    #[structopt(short, long, default_value = "mse")]
    pub comparison_mode: ComparisonMode,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            output_path: PathBuf::new(),
            window_size: 25,
            frame_skip: 0,
            key_frames_only: true,
            verbose: 0,
            comparison_mode: ComparisonMode::MSE,
        }
    }
}

impl Request {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_input_path<'a, S: AsRef<OsStr> + ?Sized>(&'a mut self, pathname: &S) -> &'a mut Self {
        self.input_path = PathBuf::from(pathname);
        self
    }

    pub fn input_path(&self) -> &Path {
        self.input_path.as_path()
    }

    pub fn set_output_path<'a, S: AsRef<OsStr> + ?Sized>(&'a mut self, pathname: &S) -> &'a mut Self {
        self.output_path = PathBuf::from(pathname);
        self
    }

    pub fn output_path(&self) -> &Path {
        self.output_path.as_path()
    }

    pub fn set_window_size<'a>(&'a mut self, window_size: u32) -> &'a mut Self {
        self.window_size = window_size;
        self
    }

    pub fn set_frame_skip<'a>(&'a mut self, frame_skip: u32) -> &'a mut Self {
        self.frame_skip = frame_skip;
        self
    }

    pub fn set_key_frames_only<'a>(&'a mut self, key_frames_only: bool) -> &'a mut Self {
        self.key_frames_only = key_frames_only;
        self
    }

    pub fn set_verbose<'a>(&'a mut self, verbose: u8) -> &'a mut Self {
        self.verbose = verbose;
        self
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ComparisonMode {
    Noop,
    Blockhash,
    GradientHash,
    MeanHash,
    MSE,
    SSIM,
}

#[derive(Debug)]
pub struct ParseComparisonModeError;

impl ToString for ParseComparisonModeError {
    fn to_string(&self) -> String {
        String::from("ParseComparisonModeError")
    }
}

impl FromStr for ComparisonMode {
    type Err = ParseComparisonModeError;

    fn from_str(s: &str) -> Result<ComparisonMode, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "noop" => Ok(ComparisonMode::Noop),
            "blockhash" => Ok(ComparisonMode::Blockhash),
            "gradienthash" => Ok(ComparisonMode::GradientHash),
            "meanhash" => Ok(ComparisonMode::MeanHash),
            "mse" => Ok(ComparisonMode::MSE),
            "ssim" => Ok(ComparisonMode::SSIM),
            _ => Err(ParseComparisonModeError),
        }
    }
}

impl Display for ComparisonMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
