use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::array;
use std::f32::{consts, EPSILON};
use std::sync::Arc;
use std::collections::HashMap;

mod editor;

const HARMONICS_COUNT: usize = 31;

pub struct Seriessynth {
    params: Arc<SeriessynthParams>,
    sample_rate: f32,
    voices: HashMap<u8, Voice>,
}

enum AHDSR {
    A,
    H,
    D,
    S,
    R
}


struct Voice {
    phase: f32,
    midi_note_freq: f32,
    midi_note_gain: Smoother<f32>,
    ahdsr: AHDSR,
    envelope: f32,
    hold: f32,
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
    pub harmonics: [ArrayParams; HARMONICS_COUNT]
}

#[derive(Params)]
struct ArrayParams {
    #[id = "noope"]
    pub nope: FloatParam,
}

#[derive(Debug, PartialEq, Enum)]
enum Waveform {
    None,
    Triangle,
    Sawtooth,
    Square,
    Noise,
}

impl Default for Seriessynth {
    fn default() -> Self {
        Self {
            params: Arc::new(SeriessynthParams::default()),
            sample_rate: 96000.0,
            voices: HashMap::new(),
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
                                min: 0.0,
                                max: util::db_to_gain(0.0),
                            },
                        )
                        .with_unit(" dB")
                        .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                        .with_string_to_value(formatters::s2v_f32_gain_to_db())
                        .with_smoother(SmoothingStyle::Logarithmic(10.0)),
                    }
                } else {
                    ArrayParams {
                        nope: FloatParam::new(
                            &format!("{:02}倍音", i + 1),
                            0.0,
                            FloatRange::Linear {
                                min: 0.0,
                                max: util::db_to_gain(0.0),
                            },
                        )
                        .with_unit(" dB")
                        .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                        .with_string_to_value(formatters::s2v_f32_gain_to_db())
                        .with_smoother(SmoothingStyle::Logarithmic(10.0)),
                    }
                }
            })
        }
    }
}

struct AhdsrValue {
    attack: f32,
    hold: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

impl Voice {
    fn calculate(&mut self, sample_rate: f32, series: &[f32; HARMONICS_COUNT], ahdsr_value: &AhdsrValue) -> f32 {
        let phase_delta = self.midi_note_freq / sample_rate;
        // let sine = (self.phase * consts::TAU).sin();
        let wave = self.wave_gen(self.phase, series);

        self.phase += phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        match self.ahdsr {
            AHDSR::A => {
                if ahdsr_value.attack < EPSILON {
                    self.ahdsr = AHDSR::H;
                    self.envelope = 1.0;
                } else {
                    self.envelope += 1.0 / (sample_rate * ahdsr_value.attack);
                    if self.envelope >= 1.0 {
                        self.envelope = 1.0;
                        self.ahdsr = AHDSR::H;
                    }
                }
            }
            AHDSR::H => {
                self.hold += 1.0 / sample_rate;
                if self.hold + 1.0 / sample_rate >= ahdsr_value.hold {
                    self.ahdsr = AHDSR::D;
                }
            }
            AHDSR::D => {
                if ahdsr_value.decay < EPSILON {
                    self.ahdsr = AHDSR::S;
                    self.envelope = ahdsr_value.sustain;
                } else {
                    self.envelope -= 1.0 / (sample_rate * ahdsr_value.decay);
                    if self.envelope <= ahdsr_value.sustain {
                        self.ahdsr = AHDSR::S;
                    }
                }
            }
            AHDSR::S => {
            }
            AHDSR::R => {
                if ahdsr_value.release < EPSILON {
                    self.envelope = 0.0;
                } else {
                    self.envelope -= 1.0 / (sample_rate * ahdsr_value.release);
                    if self.envelope <= 0.0 {
                        self.envelope = 0.0;
                    }
                }
            }
        }
        wave * self.envelope
    }

    fn wave_gen(&self, phase: f32, series: &[f32; HARMONICS_COUNT]) -> f32 {
        let mut v = 0.0;
        for i in 0..HARMONICS_COUNT {
            v += series[i] * (((i+1) as f32) * phase * consts::TAU).sin();
        }
        v
    }
}

impl Seriessynth {
    fn series(&self) -> [f32; HARMONICS_COUNT] {
        let mut series = [0.0; HARMONICS_COUNT];
        for (i, harmonic) in self.params.harmonics.iter().enumerate() {
            series[i] = harmonic.nope.smoothed.next();
        }
        series
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
        let ahdsr: AhdsrValue = AhdsrValue {
            attack: self.params.attack.smoothed.next(),
            hold: self.params.hold.smoothed.next(),
            decay: self.params.decay.smoothed.next(),
            sustain: self.params.sustain.smoothed.next(),
            release: self.params.release.smoothed.next(),
        };
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            let mut output_sample = 0.0;
            let gain = self.params.gain.smoothed.next();

            let _sine =  {

                while let Some(event) = next_event {
                    if event.timing() > sample_id as u32 {
                        break;
                    }

                    match event {
                        NoteEvent::NoteOn { note, velocity, .. } => {
                            let voice = Voice {
                                phase: 0.0,
                                midi_note_freq: util::midi_note_to_freq(note),
                                midi_note_gain: Smoother::new(SmoothingStyle::Linear(5.0)),
                                ahdsr: AHDSR::A,
                                envelope: 0.0,
                                hold: 0.0,
                            };
                            voice.midi_note_gain.set_target(self.sample_rate, velocity);
                            self.voices.insert(note, voice);
                        }
                        NoteEvent::NoteOff { note, .. } => {
                            if let Some(voice) = self.voices.get_mut(&note) {
                                voice.ahdsr = AHDSR::R;
                            }
                        }
                        NoteEvent::PolyPressure { note, pressure, .. } => {
                            if let Some(voice) = self.voices.get_mut(&note) {
                                voice.midi_note_gain.set_target(self.sample_rate, pressure);
                            }
                        }
                        _ => (),
                    }

                    next_event = context.next_event();
                }

                let series = self.series();
                for voice in self.voices.values_mut() {
                    output_sample += voice.calculate(self.sample_rate, &series, &ahdsr) * voice.midi_note_gain.next();
                }

                output_sample *= util::db_to_gain_fast(gain);
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
