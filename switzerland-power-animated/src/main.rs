use std::fs::File;
use std::io::Write;
use std::path::Path;
use switzerland_power_animated::{AnimationGenerator, PowerStatus, Result};

fn main() -> Result<()> {
    let generator = AnimationGenerator::new()?;
    let image = generator.generate(PowerStatus::Calculating {
        progress: 2,
        total: 5,
    })?;
    drop(generator);

    let mut file = File::create(Path::new("test.webp")).unwrap();
    file.write_all(&image).unwrap();
    Ok(())
}
