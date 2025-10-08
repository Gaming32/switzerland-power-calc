use crate::generator::{FramesVec, encode_frames};
use crate::{AnimationGenerator, AnimationLanguage, PowerStatus, Result};
use std::thread;
use tokio::sync::oneshot;

#[derive(Debug, Clone)]
pub struct AsyncAnimationGenerator {
    generator_send: crossbeam_channel::Sender<GenerateCommand>,
    encoder_send: crossbeam_channel::Sender<EncodeCommand>,
}

impl AsyncAnimationGenerator {
    pub async fn new() -> Result<Self> {
        let (generator_send, generator_recv) = crossbeam_channel::unbounded::<GenerateCommand>();
        let (generator_init_send, generator_init_recv) = oneshot::channel();
        thread::Builder::new()
            .name("AsyncAnimationGenerator GeneratorThread".to_string())
            .spawn(move || {
                let generator = match AnimationGenerator::new() {
                    Ok(generator) => {
                        generator_init_send
                            .send(Ok(*generator.webp_config()))
                            .unwrap();
                        generator
                    }
                    Err(e) => return generator_init_send.send(Err(e)).unwrap(),
                };
                while let Ok(command) = generator_recv.recv() {
                    let frames = generator.generate_frames(command.status, command.lang);
                    let _ = command.responder.send(frames);
                }
            })
            .expect("Failed to start GeneratorThread");
        let webp_config = generator_init_recv.await.unwrap()?;

        let encoder_threads = thread::available_parallelism().map_or(1, |x| x.get().max(3) - 2);
        let (encoder_send, encoder_recv) = crossbeam_channel::unbounded::<EncodeCommand>();
        for i in 0..encoder_threads {
            let encoder_recv = encoder_recv.clone();
            thread::Builder::new()
                .name(format!("AsyncAnimationGenerator EncoderThread-{i}"))
                .spawn(move || {
                    while let Ok(command) = encoder_recv.recv() {
                        let image = encode_frames(command.frames, &webp_config).map(|x| x.to_vec());
                        let _ = command.responder.send(image);
                    }
                })
                .expect("Failed to start EncoderThread");
        }

        Ok(Self {
            generator_send,
            encoder_send,
        })
    }

    pub async fn generate(&self, status: PowerStatus, lang: AnimationLanguage) -> Result<Vec<u8>> {
        let (frames_send, frames_recv) = oneshot::channel();
        self.generator_send
            .send(GenerateCommand {
                status,
                lang,
                responder: frames_send,
            })
            .unwrap();
        let frames = frames_recv.await.unwrap()?;

        let (image_send, image_recv) = oneshot::channel();
        self.encoder_send
            .send(EncodeCommand {
                frames,
                responder: image_send,
            })
            .unwrap();
        image_recv.await.unwrap()
    }
}

struct GenerateCommand {
    status: PowerStatus,
    lang: AnimationLanguage,
    responder: oneshot::Sender<Result<FramesVec>>,
}

struct EncodeCommand {
    frames: FramesVec,
    responder: oneshot::Sender<Result<Vec<u8>>>,
}
