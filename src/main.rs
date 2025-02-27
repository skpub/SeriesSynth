use nih_plug::prelude::*;

use seriessynth::Seriessynth;

fn main() {
    nih_export_standalone::<Seriessynth>();
}
