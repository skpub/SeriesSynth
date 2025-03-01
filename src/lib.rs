use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::array;
use std::env::current_exe;
use std::f32::{consts, EPSILON};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};
use std::thread::current;

mod editor;

const HARMONICS_COUNT: usize = 31;

pub struct Seriessynth {
    params: Arc<SeriessynthParams>,
    sample_rate: f32,
    voices: HashMap<u8, VecDeque<Voice>>,
}

enum AHDSR {
    A,
    H,
    D,
    S,
    R,
    DEAD,
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
        let mut final_wave = 0.0;
        for voice_queue in self.voices.values_mut() {
            if voice_queue.len() == 0 {
                continue;
            }
            let mut kill = false;
            for voice in voice_queue.iter_mut() {
                let phase_delta = voice.midi_note_freq / self.sample_rate;
                let mut wave = 0.0;
                for i in 0..HARMONICS_COUNT {
                    wave += series[i] * (((i+1) as f32) * voice.phase * consts::TAU).sin();
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
                final_wave += wave * voice.envelope;
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
