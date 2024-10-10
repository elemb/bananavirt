use std::sync::Arc;
use parking_lot::RwLock;
use uuid::Uuid;

pub type ModuleId = Uuid;

pub trait Module: Send + Sync {
    fn process(&mut self, inputs: &[f32], outputs: &mut [f32]);
    fn id(&self) -> ModuleId;
    fn name(&self) -> &str;
}

pub struct Port {
    pub buffer: Arc<RwLock<Vec<f32>>>,
}

impl Port {
    pub fn new(buffer_size: usize) -> Self {
        Port {
            buffer: Arc::new(RwLock::new(vec![0.0; buffer_size])),
        }
    }
}

pub struct ModuleBase {
    pub id: ModuleId,
    pub name: String,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
}

impl ModuleBase {
    pub fn new(name: &str, input_count: usize, output_count: usize, buffer_size: usize) -> Self {
        ModuleBase {
            id: Uuid::new_v4(),
            name: name.to_string(),
            inputs: (0..input_count).map(|_| Port::new(buffer_size)).collect(),
            outputs: (0..output_count).map(|_| Port::new(buffer_size)).collect(),
        }
    }

    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
