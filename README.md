# timelapse-rs

Inspired by the method outlined in [Reps, 2018: Smooth 3D printing timelapse by optimal frame selection][1].
The differences are in which algorithm is used to compute the similarity index, and that this
version could potentially benefit from native performance and parallelization offered by Rust. It
also does not depend on OpenCV which is, let's face it, enormous - the only DLL dependency is ffmpeg,
and that's already quite considerable.

The next step would be turning this into a GUI app.

[1]: https://reps.cc/?p=85

## Demo

The demo was produced from a timelapse of a 5 hour print with an interval of 1 second, resulting in
a 5 minute input video at 30fps. The input video was then run through the `mse` (mean squared error)
frame selection strategy and a window size of 30, resulting in one frame of output being picked
from one second of input. With the output video set to 30fps as well, this results in a final
timelapse of around 20s. The control video uses the `noop` strategy which just picks the first frame
in every window. All videos are at 1440p as that's the smallest picture size my camera would go down
to in still mode and at 16:9 aspect ratio.

* MSE: https://youtu.be/UnMuDfd5rGw
* Control: https://youtu.be/NP9ql0ujFtQ

## Licence

For now, `timelapse-rs` is licenced under GPL version 3. This is largely because it uses ffmpeg
which itself is licenced under either LGPLv3 or GPLv3. For now `timelapse-rs` is built against
the LGPL version of the DLLs, however that means it can't support h264 video. I am considering
either dropping down to LGPLv3 (and I won't be able to go any more permissive than that
without violating ffmpeg's licences) or using the GPL version of ffmpeg and offering h246 support
that way.

```
    Copyright © 2020 Filip Wieland

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
