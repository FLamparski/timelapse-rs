# timelapse-rs

Inspired by the method outlined in [Reps, 2018: Smooth 3D printing timelapse by optimal frame selection][1].
The differences are in which algorithm is used to compute the similarity index, and that this
version could potentially benefit from native performance and parallelization offered by Rust. It
also does not depend on OpenCV which is, let's face it, enormous - the only DLL dependency is ffmpeg,
and that's already quite considerable.

The next step would be turning this into a GUI app.

[1]: https://reps.cc/?p=85