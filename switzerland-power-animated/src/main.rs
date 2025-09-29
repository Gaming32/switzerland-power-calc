use std::fs::File;
use std::io::Write;
use std::path::Path;
use switzerland_power_animated::MatchOutcome::{Lose, Unplayed, Win};
use switzerland_power_animated::{AnimationGenerator, AnimationLanguage, PowerStatus, Result};

fn main() -> Result<()> {
    let generator = AnimationGenerator::new()?;
    // let image = generator.generate(PowerStatus::Calculated {
    //     calculation_rounds: 4,
    //     power: 1743.2,
    //     rank: 34,
    // }, AnimationLanguage::USen)?;
    let image = generator.generate(
        PowerStatus::SetPlayed {
            matches: [Win, Lose, Win, Win, Unplayed],
            old_power: 1743.2,
            new_power: 1792.8,
            old_rank: 34,
            new_rank: 28,
        },
        AnimationLanguage::USen,
    )?;
    drop(generator);

    let mut file = File::create(Path::new("test.webp")).unwrap();
    file.write_all(&image).unwrap();
    Ok(())
}
