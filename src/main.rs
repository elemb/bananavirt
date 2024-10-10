mod module;
mod engine;
mod serge_modules;
mod envelope_generator;

use crate::engine::AudioEngine;
use crate::serge_modules::{SergeVCO, SergeVCF};
use crate::envelope_generator::EnvelopeGenerator;
use std::sync::Arc;
use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam::channel::{bounded, Receiver, Sender};
use std::io::{self, BufRead};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 44100.0;
    let buffer_size = 512;

    let mut engine = AudioEngine::new(sample_rate, buffer_size);

    // Add modules
    let vco = SergeVCO::new(sample_rate as f32);
    let vcf = SergeVCF::new(sample_rate as f32);
    let eg = EnvelopeGenerator::new(sample_rate as f32);

    let vco_id = engine.add_module(vco);
    let vcf_id = engine.add_module(vcf);
    let eg_id = engine.add_module(eg);

    // Connect modules
    engine.connect_modules(vco_id, 0, vcf_id, 0); // VCO output to VCF input
    engine.connect_modules(eg_id, 0, vco_id, 1);  // EG output to VCO FM input
    engine.connect_modules(eg_id, 0, vcf_id, 2);  // EG output to VCF EG input

    // Set up audio output
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config()?;

    let engine = Arc::new(parking_lot::Mutex::new(engine));

    // Spawn a thread to handle user input
    let engine_clone = engine.clone();
    thread::spawn(move || {
        handle_user_input(engine_clone, vco_id, vcf_id, eg_id);
    });

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), engine),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), engine),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), engine),
        _ => Err("Unsupported sample format".into()),
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    engine: Arc<parking_lot::Mutex<AudioEngine>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: cpal::Sample,
{
    let (tx, rx) = bounded::<Vec<f32>>(2);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, &rx)
        },
        |err| eprintln!("An error occurred on the output audio stream: {}", err),
        None,
    )?;

    stream.play()?;

    println!("Audio engine started. Enter commands to control parameters.");
    println!("Available commands:");
    println!("  vco freq <value>  - Set VCO frequency (20-20000 Hz)");
    println!("  vcf cutoff <value> - Set VCF cutoff frequency (20-20000 Hz)");
    println!("  eg attack <value> - Set EG attack time (0.001-10 seconds)");
    println!("  eg decay <value>  - Set EG decay time (0.001-10 seconds)");
    println!("  eg sustain <value> - Set EG sustain level (0-1)");
    println!("  eg release <value> - Set EG release time (0.001-10 seconds)");
    println!("  note on           - Trigger note on");
    println!("  note off          - Trigger note off");
    println!("  quit              - Exit the program");

    loop {
        let output = engine.lock().process();
        tx.send(output).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn write_data<T>(output: &mut [T], rx: &Receiver<Vec<f32>>)
where
    T: cpal::Sample,
{
    if let Ok(buffer) = rx.try_recv() {
        for (out, sample) in output.iter_mut().zip(buffer.iter().cycle()) {
            *out = T::from::<f32>(*sample);
        }
    }
}

fn handle_user_input(
    engine: Arc<parking_lot::Mutex<AudioEngine>>,
    vco_id: module::ModuleId,
    vcf_id: module::ModuleId,
    eg_id: module::ModuleId,
) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(input) = line {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() == 3 {
                match parts[0] {
                    "vco" => {
                        if parts[1] == "freq" {
                            if let Ok(freq) = parts[2].parse::<f32>() {
                                if let Some(module) = engine.lock().get_module(vco_id) {
                                    if let Some(vco) = module.lock().downcast_mut::<SergeVCO>() {
                                        vco.set_frequency(freq);
                                        println!("VCO frequency set to {} Hz", freq);
                                    }
                                }
                            }
                        }
                    }
                    "vcf" => {
                        if parts[1] == "cutoff" {
                            if let Ok(cutoff) = parts[2].parse::<f32>() {
                                if let Some(module) = engine.lock().get_module(vcf_id) {
                                    if let Some(vcf) = module.lock().downcast_mut::<SergeVCF>() {
                                        vcf.set_cutoff(cutoff);
                                        println!("VCF cutoff set to {} Hz", cutoff);
                                    }
                                }
                            }
                        }
                    }
                    "eg" => {
                        if let Some(module) = engine.lock().get_module(eg_id) {
                            if let Some(eg) = module.lock().downcast_mut::<EnvelopeGenerator>() {
                                match parts[1] {
                                    "attack" => {
                                        if let Ok(value) = parts[2].parse::<f32>() {
                                            eg.set_attack(value);
                                            println!("EG attack set to {} seconds", value);
                                        }
                                    }
                                    "decay" => {
                                        if let Ok(value) = parts[2].parse::<f32>() {
                                            eg.set_decay(value);
                                            println!("EG decay set to {} seconds", value);
                                        }
                                    }
                                    "sustain" => {
                                        if let Ok(value) = parts[2].parse::<f32>() {
                                            eg.set_sustain(value);
                                            println!("EG sustain set to {}", value);
                                        }
                                    }
                                    "release" => {
                                        if let Ok(value) = parts[2].parse::<f32>() {
                                            eg.set_release(value);
                                            println!("EG release set to {} seconds", value);
                                        }
                                    }
                                    _ => println!("Unknown EG parameter"),
                                }
                            }
                        }
                    }
                    _ => println!("Unknown command"),
                }
            } else if parts.len() == 2 && parts[0] == "note" {
                if let Some(module) = engine.lock().get_module(eg_id) {
                    if let Some(eg) = module.lock().downcast_mut::<EnvelopeGenerator>() {
                        match parts[1] {
                            "on" => {
                                eg.trigger_on();
                                println!("Note on");
                            }
                            "off" => {
                                eg.trigger_off();
                                println!("Note off");
                            }
                            _ => println!("Invalid note command"),
                        }
                    }
                }
            } else if input.trim() == "quit" {
                println!("Exiting...");
                std::process::exit(0);
            } else {
                println!("Invalid command format");
            }
        }
    }
}
