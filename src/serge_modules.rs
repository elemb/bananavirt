use crate::module::{Module, ModuleBase, ModuleId};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct SergeVCO {
    base: ModuleBase,
    frequency: f32,
    fm_amount: f32,
    phase: f32,
}

impl SergeVCO {
    pub fn new(sample_rate: f32) -> Arc<Mutex<dyn Module>> {
        Arc::new(Mutex::new(Self {
            base: ModuleBase::new("Serge VCO", 2, 1, 512), // FM input, EG input, output
            frequency: 440.0,
            fm_amount: 1.0,
            phase: 0.0,
        }))
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq.max(20.0).min(20000.0);
    }

    pub fn set_fm_amount(&mut self, amount: f32) {
        self.fm_amount = amount.max(0.0).min(1.0);
    }
}

impl Module for SergeVCO {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        let fm_input = inputs[0] * self.fm_amount;
        let eg_input = inputs[1];
        let frequency = self.frequency * (1.0 + fm_input) * (1.0 + eg_input);
        
        for output in outputs.iter_mut() {
            self.phase += frequency / 44100.0; // Assuming 44.1kHz sample rate
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            *output = (self.phase * 2.0 - 1.0) * 5.0; // -5V to 5V output
        }
    }

    fn id(&self) -> ModuleId {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }
}

pub struct SergeVCF {
    base: ModuleBase,
    cutoff: f32,
    resonance: f32,
    last_output: f32,
}

impl SergeVCF {
    pub fn new(sample_rate: f32) -> Arc<Mutex<dyn Module>> {
        Arc::new(Mutex::new(Self {
            base: ModuleBase::new("Serge VCF", 3, 1, 512), // Audio input, cutoff CV input, EG input, output
            cutoff: 1000.0,
            resonance: 0.5,
            last_output: 0.0,
        }))
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.cutoff = cutoff.max(20.0).min(20000.0);
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.max(0.0).min(0.99);
    }
}

impl Module for SergeVCF {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]) {
        let input = inputs[0];
        let cutoff_cv = inputs[1];
        let eg_input = inputs[2];
        
        // Simple one-pole lowpass filter implementation
        let cutoff = (self.cutoff * (1.0 + cutoff_cv) * (1.0 + eg_input)).max(20.0).min(20000.0);
        let alpha = (2.0 * std::f32::consts::PI * cutoff / 44100.0).tan() / (2.0 * std::f32::consts::PI * cutoff / 44100.0).tan() + 1.0;
        
        for output in outputs.iter_mut() {
            self.last_output = self.last_output + alpha * (input - self.last_output);
            *output = self.last_output * 5.0; // Scale to -5V to 5V range
        }
    }

    fn id(&self) -> ModuleId {
        self.base.id()
    }

    fn name(&self) -> &str {
        self.base.name()
    }
}
