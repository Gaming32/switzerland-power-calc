use std::fs::File;
use std::io::Write;
use std::path::Path;
use switzerland_power_animated::{AnimationGenerator, PowerStatus, Result};

fn main() -> Result<()> {
    let generator = AnimationGenerator::new()?;
    let image = generator.generate(PowerStatus::Calculated {
        calculation_rounds: 4,
        power: 1743.2,
        rank: 34,
    })?;
    drop(generator);

    let mut file = File::create(Path::new("test.webp")).unwrap();
    file.write_all(&image).unwrap();
    Ok(())
}
