use nih_plug::prelude::*;
use nih_plug::params::enums::Enum;
use nih_plug_vizia::ViziaState;
use std::array;
use std::f32::{consts, EPSILON};
use std::fmt::Debug;
use std::sync::Arc;
use std::collections::{HashMap, VecDeque};

mod editor;

const HARMONICS_COUNT: usize = 31;

pub struct Seriessynth {
    params: Arc<SeriessynthParams>,
    sample_rate: f32,
    voices: HashMap<u8, VecDeque<Voice>>,
    lfo_phase: f32,
}

enum AHDSR {
    A,
    H,
    D,
    S,
    R,
    DEAD,
}

#[derive(Debug, PartialEq, Enum)]
enum Waveform {
    None,
    Triangle,
    Sawtooth,
    Square,
}

#[derive(Debug, PartialEq, Enum)]
enum LfoDest {
    None,
    Phase,
    Gain,
}

#[derive(Debug, PartialEq)]
enum AmpWidth {
    One,
    N,
    N2,
}

impl Enum for AmpWidth {
    fn variants() -> &'static [&'static str] {
        &["1", "1/N", "1/N^2"]
    }

    fn ids() -> Option<&'static [&'static str]> {
        Some(&["1", "N", "N2"])
    }
    fn to_index(self) -> usize {
        match self {
            AmpWidth::One => 0,
            AmpWidth::N => 1,
            AmpWidth::N2 => 2,
        }
    }
    fn from_index(index: usize) -> Self {
        match index {
            0 => AmpWidth::One,
            1 => AmpWidth::N,
            2 => AmpWidth::N2,
            _ => panic!("Invalid index"),
        }
    }
}


struct Voice {
    phase: f32,
    midi_note_freq: f32,
    midi_note_gain: Smoother<f32>,
    ahdsr: AHDSR,
    envelope: f32,
    hold: f32,
    dead: f32,
}

#[derive(Params)]
struct SeriessynthParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "gain"]
    pub gain: FloatParam,

    #[id = "freq"]
    pub frequency: FloatParam,

    #[id = "higherwaveform"]
    pub higher_waveform: EnumParam<Waveform>,

    #[id = "A"]
    pub attack: FloatParam,

    #[id = "H"]
    pub hold: FloatParam,

    #[id = "D"]
    pub decay: FloatParam,

    #[id = "S"]
    pub sustain: FloatParam,

    #[id = "R"]
    pub release: FloatParam,

    #[nested(array, group= "harmonics")]
    pub harmonics: [ArrayParams; HARMONICS_COUNT],

    #[id = "ampwidth"]
    pub amp_width: EnumParam<AmpWidth>,

    #[id = "noise"]
    pub noise: FloatParam,

    #[id = "Base Freq factor"]
    pub base_freq_factor: IntParam,

    #[id = "Base Freq inverse factor"]
    pub base_freq_inverse_factor: IntParam,

    #[id = "+ N Cent"]
    pub plus_n_cent: IntParam,

    #[id = "LFO_freq"]
    pub lfo: FloatParam,

    #[id = "LFO amp"]
    pub lfo_amp: FloatParam,

    #[id = "LFO dest"]
    pub lfo_dest: EnumParam<LfoDest>,
}

#[derive(Params)]
struct ArrayParams {
    #[id = "noope"]
    pub nope: FloatParam,
}

impl Default for Seriessynth {
    fn default() -> Self {
        Self {
            params: Arc::new(SeriessynthParams::default()),
            sample_rate: 96000.0,
            voices: HashMap::new(),
            lfo_phase: 0.0,
        }
    }
}

impl Default for SeriessynthParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            gain: FloatParam::new(
                "Gain",
                -10.0,
                FloatRange::Linear {
                    min: -30.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_step_size(0.01)
            .with_unit(" dB"),
            frequency: FloatParam::new(
                "Frequency",
                440.0,
                FloatRange::Skewed {
                    min: 1.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(3.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            higher_waveform: EnumParam::new("Base Waveform", Waveform::None),
            attack: FloatParam::new(
                "Attack",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            hold: FloatParam::new(
                "Hold",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            decay: FloatParam::new(
                "Decay",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            sustain: FloatParam::new(
                "Sustain",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            release: FloatParam::new(
                "Release",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            harmonics: array::from_fn(|i| {
                if i == 0 {
                    ArrayParams {
                        nope: FloatParam::new(
                            "1倍音",
                            1.0,
                            FloatRange::Linear {
                                min: -1.0,
                                max: 1.0,
                            },
                        )
                        .with_smoother(SmoothingStyle::Linear(10.0)),
                    }
                } else {
                    ArrayParams {
                        nope: FloatParam::new(
                            &format!("{:02}倍音", i + 1),
                            0.0,
                            FloatRange::Linear {
                                min: -1.0,
                                max: 1.0,
                            },
                        )
                        .with_smoother(SmoothingStyle::Linear(10.0)),
                    }
                }
            }),
            amp_width: EnumParam::new("倍音係数", AmpWidth::One),
            noise: FloatParam::new(
                "Noise",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),
            base_freq_factor: IntParam::new(
                "Base Freq factor",
                1,
                IntRange::Linear {
                    min: 1,
                    max: 23,
                },
            ),
            base_freq_inverse_factor: IntParam::new(
                "Base Freq inverse factor",
                1,
                IntRange::Linear {
                    min: 1,
                    max: 23,
                },
            ),
            plus_n_cent: IntParam::new(
                "+ N Cent",
                0,
                IntRange::Linear {
                    min: -100,
                    max: 100,
                },
            ),
            lfo: FloatParam::new(
                "LFO",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            ).with_unit(" Hz"),
            lfo_amp: FloatParam::new(
                "LFO amp",
                1.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),
            lfo_dest: EnumParam::new("LFO dest", LfoDest::None),
        }
    }
}

impl Seriessynth {
    fn series(&self) -> [f32; HARMONICS_COUNT] {
        let current_params_ref = Arc::clone(&self.params);
        let mut series = [0.0; HARMONICS_COUNT];
        for (i, harmonic) in current_params_ref.harmonics.iter().enumerate() {
            series[i] = harmonic.nope.smoothed.next();
        }
        series
    }
}

impl Seriessynth {
    fn calculate(&mut self) -> f32 {
        let series = self.series();
        let params = Arc::clone(&self.params);
        let higher_waveform = params.higher_waveform.value();
        let amp_width = params.amp_width.value();
        let noise = params.noise.smoothed.next();
        let base_freq_factor = params.base_freq_factor.smoothed.next();
        let base_freq_inverse_factor = params.base_freq_inverse_factor.smoothed.next();
        let plus_n_cent = params.plus_n_cent.smoothed.next();
        let lfo_hz = params.lfo.smoothed.next();
        let lfo_amp = params.lfo_amp.smoothed.next();
        let lfo_dest = params.lfo_dest.value();

        let lfo_phase_delta = lfo_hz / self.sample_rate;
        self.lfo_phase = (self.lfo_phase + lfo_phase_delta) % 1.0;
        let lfo_phase_mod = if lfo_dest == LfoDest::Phase {
            1.0 + lfo_amp * (self.lfo_phase * consts::TAU).sin()
        } else {
            1.0
        };
        let lfo_gain_mod = if lfo_dest == LfoDest::Gain {
            1.0 + lfo_amp * (self.lfo_phase * consts::TAU).sin()
        } else {
            1.0
        };

        let cent_factor = 2f32.powf(plus_n_cent as f32 / 1200.0);
        let freq_factor = (base_freq_factor as f32) / (base_freq_inverse_factor as f32);


        let mut final_wave = 0.0;
        for voice_queue in self.voices.values_mut() {
            if voice_queue.len() == 0 {
                continue;
            }
            let mut kill = false;
            for voice in voice_queue.iter_mut() {
                let phase_delta = (voice.midi_note_freq * cent_factor * freq_factor * lfo_phase_mod) / self.sample_rate;
                let mut wave = 0.0;
                for i in 0..HARMONICS_COUNT {
                    wave +=  match amp_width {
                            AmpWidth::One => 1.0,
                            AmpWidth::N => 1.0 / (i as f32 + 1.0),
                            AmpWidth::N2 => 1.0 / (((i as f32 + 1.0) * (i as f32 + 1.0)))
                        } * series[i] * (((i+1) as f32) * voice.phase * consts::TAU).sin();
                }
                let nyquist_index = ((self.sample_rate as f32) / (voice.midi_note_freq * freq_factor * cent_factor as f32)).floor() as usize;
                if higher_waveform == Waveform::Square {
                    for i in (HARMONICS_COUNT >> 1)..(nyquist_index >> 1) {
                        wave += (1.0 / (2.0 * i as f32) as f32)
                            * ((i as f32) * voice.phase * consts::TAU).sin();
                    }
                }
                if higher_waveform == Waveform::Triangle {
                    for i in (HARMONICS_COUNT>>1)..(nyquist_index >> 1) {
                        wave += if i % 2 == 0 {-1.0} else {1.0} * (1.0 / (2.0 * i as f32)) * (1.0 / (2.0 * i as f32))
                            * ((i as f32) * voice.phase * consts::TAU).sin();
                    }
                }
                if higher_waveform == Waveform::Sawtooth {
                    for i in HARMONICS_COUNT+1..nyquist_index {
                        wave += (1.0 / i as f32)
                            * ((i as f32) * voice.phase * consts::TAU).sin();
                    }
                }
                if noise > EPSILON {
                    let f: f32 = rand::random_range(-noise..noise);
                    wave += f;
                }
                voice.phase += phase_delta;
                if voice.phase >= 1.0 {
                    voice.phase -= 1.0;
                }
                match voice.ahdsr {
                    AHDSR::A => {
                        if self.params.attack.smoothed.next() < EPSILON {
                            voice.ahdsr = AHDSR::H;
                            voice.envelope = 1.0;
                        } else {
                            voice.envelope += 1.0 / (self.sample_rate * self.params.attack.smoothed.next());
                            if voice.envelope >= 1.0 {
                                voice.envelope = 1.0;
                                voice.ahdsr = AHDSR::H;
                            }
                        }
                    }
                    AHDSR::H => {
                        voice.hold += 1.0 / self.sample_rate;
                        if voice.hold + 1.0 / self.sample_rate >= self.params.hold.smoothed.next() {
                            voice.ahdsr = AHDSR::D;
                        }
                    }
                    AHDSR::D => {
                        if self.params.decay.smoothed.next() < EPSILON {
                            voice.ahdsr = AHDSR::S;
                            voice.envelope = self.params.sustain.smoothed.next();
                        } else {
                            voice.envelope -= 1.0 / (self.sample_rate * self.params.decay.smoothed.next());
                            if voice.envelope <= self.params.sustain.smoothed.next() {
                                voice.ahdsr = AHDSR::S;
                            }
                        }
                    }
                    AHDSR::S => {

                    }
                    AHDSR::R => {
                        if self.params.release.smoothed.next() < EPSILON {
                            voice.envelope = 0.0;
                            kill = true;
                        } else {
                            voice.envelope -= 1.0 / (self.sample_rate * self.params.release.smoothed.next());
                            if voice.envelope <= 0.0 {
                                voice.envelope = 0.0;
                                kill = true;
                            }
                        }
                    }
                    AHDSR::DEAD => {
                        if self.params.release.smoothed.next() < EPSILON {
                            voice.envelope = 0.0;
                            kill = true;
                        } else {
                            voice.envelope -= 1.0 / (self.sample_rate * self.params.release.smoothed.next());
                            if voice.envelope <= 0.0 {
                                voice.envelope = 0.0;
                                kill = true;
                            }
                        }
                    }
                }
                final_wave += wave * voice.envelope * lfo_gain_mod;
            }
            if kill {
                voice_queue.pop_back();
            }
        }
        final_wave
    }
}

impl Plugin for Seriessynth {
    const NAME: &'static str = "SeriesSynth";
    const VENDOR: &'static str = "skpub";
    const URL: &'static str = "none";
    const EMAIL: &'static str = "satodeyannsu@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        self.voices.clear();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            let gain = self.params.gain.smoothed.next();

            let output_sample;
            let _sine =  {
                while let Some(event) = next_event {
                    if event.timing() > sample_id as u32 {
                        break;
                    }

                    match event {
                        NoteEvent::NoteOn { note, velocity, .. } => {
                            
                            // If the note is already playing, begin the kill phase.
                            match self.voices.get_mut(&note) {
                                Some(voice_queue) => {
                                    let voice = voice_queue.get_mut(0);
                                    match voice {
                                        Some(voice) => {
                                            voice.ahdsr = AHDSR::DEAD;
                                            voice.dead = 0.0;
                                        }
                                        None => (),
                                    }
                                }
                                None => (),
                            }
                            let voice = Voice {
                                phase: 0.0,
                                midi_note_freq: util::midi_note_to_freq(note),
                                midi_note_gain: Smoother::new(SmoothingStyle::Linear(5.0)),
                                ahdsr: AHDSR::A,
                                envelope: 0.0,
                                hold: 0.0,
                                dead: 0.0,
                            };
                            voice.midi_note_gain.set_target(self.sample_rate, velocity);
                            let queue = self.voices.entry(note).or_insert_with(VecDeque::new);
                            queue.push_front(voice);
                        }
                        NoteEvent::NoteOff { note, .. } => {
                            if let Some(voice_queue) = self.voices.get_mut(&note) {
                                voice_queue.get_mut(0).unwrap().ahdsr = AHDSR::R;
                            }
                        }
                        NoteEvent::PolyPressure { note, pressure, .. } => {
                            if let Some(voice_queue) = self.voices.get_mut(&note) {
                                match voice_queue.get_mut(0) {
                                    Some(voice) => {
                                        voice.midi_note_gain.set_target(self.sample_rate, pressure);
                                    }
                                    None => (),
                                }
                            }
                        }
                        _ => (),
                    }

                    next_event = context.next_event();
                }

                // for voice in self.voices.values_mut() {
                //     // output_sample += voice.calculate(self.sample_rate, &series, &ahdsr) * voice.midi_note_gain.next();
                //     output_sample += self.calculate(voice) * voice.midi_note_gain.next();
                // }
                output_sample = self.calculate() * util::db_to_gain_fast(gain);
            };

            for sample in channel_samples {
                *sample = output_sample;
            }
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Seriessynth {
    const CLAP_ID: &'static str = "org.sk-dev.seriessynth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("series synth");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Seriessynth {
    const VST3_CLASS_ID: [u8; 16] = *b"SeriesSynth     ";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument];
}


nih_export_clap!(Seriessynth);
nih_export_vst3!(Seriessynth);
