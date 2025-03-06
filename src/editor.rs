use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::{SeriessynthParams, HARMONICS_COUNT};

#[derive(Lens)]
struct Data {
    params: Arc<SeriessynthParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (750, 850))
}

pub(crate) fn create(
    params: Arc<SeriessynthParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);

        Data {
            params: params.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "SeriesSynth")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Light)
                .font_size(30.0)
                .height(Pixels(50.0))
                .width(Pixels(600.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(10.0))
                .text_align(TextAlign::Center);

            HStack::new(cx, |cx| {
                VStack::new(cx, |cx| {
                    for i in 0..HARMONICS_COUNT {
                        HStack::new(cx, |cx| {
                            let index = i;
                            let label_text = format!("{:02}倍音", i + 1);
                            Label::new(cx, &label_text)
                                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                                .font_weight(FontWeightKeyword::Light)
                                .font_size(20.0)
                                .height(Pixels(20.0))
                                .child_top(Stretch(1.0))
                                .child_bottom(Pixels(0.0));
                            ParamSlider::new(cx, Data::params, move |params| &params.harmonics[index].nope)
                                .height(Pixels(25.0))
                                .width(Stretch(1.0));
                        })
                        .width(Pixels(300.0));
                    }
                })
                .row_between(Pixels(5.0))
                .width(Pixels(320.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0));

                VStack::new(cx, |cx| {
                    Label::new(cx, "Gain")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.gain);

                    Label::new(cx, "Attack")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.attack);

                    Label::new(cx, "Hold")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.hold);

                    Label::new(cx, "Decay")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.decay);

                    Label::new(cx, "Sustain")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.sustain);

                    Label::new(cx, "Release")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.release);

                    Label::new(cx, &format!("{}倍音より上の波形", HARMONICS_COUNT))
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.higher_waveform);

                    Label::new(cx, "倍音係数")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.amp_width);

                    Label::new(cx, "Noise")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.noise);

                    Label::new(cx, "Base freq factor")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.base_freq_factor);

                    Label::new(cx, "Base freq inverse factor")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.base_freq_inverse_factor);

                    Label::new(cx, "Plus N Cent")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.plus_n_cent);
                })
                .row_between(Pixels(0.0))
                .width(Pixels(200.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0));

                VStack::new(cx, |cx| {
                    Label::new(cx, "LFO Freq")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.lfo);

                    Label::new(cx, "LFO Dest")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.lfo_dest);

                    Label::new(cx, "LFO Amp")
                        .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                        .font_weight(FontWeightKeyword::Light)
                        .font_size(20.0)
                        .height(Pixels(25.0))
                        .child_top(Stretch(1.0))
                        .child_bottom(Pixels(0.0));
                    ParamSlider::new(cx, Data::params, |params| &params.lfo_amp);
                })
                .row_between(Pixels(0.0))
                .width(Pixels(200.0))
                .child_left(Stretch(1.0))
                .child_right(Stretch(1.0));
            });
        });

        ResizeHandle::new(cx);
    })
}
