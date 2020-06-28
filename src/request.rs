use std::path::{Path, PathBuf};
use std::ffi::OsStr;

pub struct Request {
    input_path: PathBuf,
    output_path: PathBuf,
    pub window_size: u32,
    pub frame_skip: u32,
    pub key_frames_only: bool,
    pub verbose: bool,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            output_path: PathBuf::new(),
            window_size: 25,
            frame_skip: 0,
            key_frames_only: true,
            verbose: false,
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

    pub fn set_verbose<'a>(&'a mut self, verbose: bool) -> &'a mut Self {
        self.verbose = verbose;
        self
    }
}
