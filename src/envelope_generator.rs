use crate::module::{Module, ModuleBase, ModuleId};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct EnvelopeGenerator {
    base: ModuleBase,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    stage: EnvelopeStage,
    current_level: f32,
    gate: bool,
}

enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl EnvelopeGenerator {
    pub fn new(sample_rate: f32) -> Arc<Mutex<dyn Module>> {
        Arc::new(Mutex::new(Self {
            base: ModuleBase::new("Envelope Generator", 1, 1, 512), // Gate input, envelope output
            attack: 0.01,
            decay: 0.1,
            sustain: 0.5,
            release: 0.2,
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            gate: false,
        }))
    }

    pub fn set_attack(&mut self, attack: f32) {
        self.attack = attack.max(0.001).min(10.0);
    }

    pub fn set_decay(&mut self, decay: f32) {
        self.decay = decay.max(0.001).min(10.0);
    }

    pub fn set_sustain(&mut self, sustain: f32) {
        self.sustain = sustain.max(0.0).min(1.0);
    }

    pub fn set_release(&mut self, release: f32) {
        self.release = release.max(0.001).min(10.0);
    }

    pub fn trigger_on(&mut self) {
        self.gate = true;
        self.stage = EnvelopeStage::Attack;
    }

    pub fn trigger_off(&mut self) {
        self.gate = false;
        self.stage = EnvelopeStage::Release;
    }
}

impl Module for EnvelopeGenerator {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        let sample_time = 1.0 / 44100.0; // Assuming 44.1kHz sample rate

        for output in outputs.iter_mut() {
            match self.stage {
                EnvelopeStage::Idle => {
                    self.current_level = 0.0;
                }
                EnvelopeStage::Attack => {
                    self.current_level += sample_time / self.attack;
                    if self.current_level >= 1.0 {
                        self.current_level = 1.0;
                        self.stage = EnvelopeStage::Decay;
                    }
                }
                EnvelopeStage::Decay => {
                    self.current_level -= sample_time / self.decay * (1.0 - self.sustain);
                    if self.current_level <= self.sustain {
                        self.current_level = self.sustain;
                        self.stage = EnvelopeStage::Sustain;
                    }
                }
                EnvelopeStage::Sustain => {
                    self.current_level = self.sustain;
                }
                EnvelopeStage::Release => {
                    self.current_level -= sample_time / self.release * self.sustain;
                    if self.current_level <= 0.0 {
                        self.current_level = 0.0;
                        self.stage = EnvelopeStage::Idle;
                    }
                }
            }

            *output = self.current_level * 5.0; // Scale to 0-5V range
        }
    }

    fn id(&self) -> ModuleId {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }
}
