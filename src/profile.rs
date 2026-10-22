use crate::{
    cli::ProfileArgs,
    config::{self, Office},
    ctx::Ctx,
};
use anyhow::Result;

pub fn run(ctx: &Ctx, args: ProfileArgs) -> Result<()> {
    let mut config = config::load(&ctx.paths)?;
    let mut changed = false;
    if let Some(value) = args.developer {
        config.profile.is_developer = value;
        changed = true;
    }
    if let Some(value) = args.part_time {
        config.profile.is_part_time = value;
        changed = true;
    }
    if let Some(value) = args.office {
        config.profile.office = match value.to_lowercase().as_str() {
            "none" | "unset" => None,
            "hacettepe" => Some(Office::Hacettepe),
            "izmir" | "i̇zmir" => Some(Office::Izmir),
            "odtu" | "odtü" => Some(Office::Odtu),
            "istanbul" => Some(Office::Istanbul),
            _ => anyhow::bail!(
                "Unknown office: {value}. Use hacettepe, izmir, odtu, istanbul, or none."
            ),
        };
        changed = true;
    }
    if changed {
        config::save(&ctx.paths, &config)?;
    }
    println!(
        "Developer: {}\nOffice: {}\nPart-time: {}",
        config.profile.is_developer,
        config
            .profile
            .office
            .map(Office::label)
            .unwrap_or("Not selected"),
        config.profile.is_part_time
    );
    Ok(())
}
