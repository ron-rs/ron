use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

#[path = "lib.rs"]
mod typed_data;

fn main() -> anyhow::Result<()> {
    let Cli { corpus } = Cli::parse();

    if !corpus.is_dir() {
        anyhow::bail!("{corpus:?} is not a directory");
    }

    for entry in fs::read_dir(&corpus).context("could not iterate over corpus")? {
        let path = entry.context("invalid corpus entry")?.path();
        let data = fs::read(&path).context("could not read corpus entry")?;

        if let Some(typed_data) = typed_data::roundtrip_arbitrary_typed_ron_or_panic(&data) {
            println!(
                "{path:?}\n{}\n",
                ron::ser::to_string_pretty(
                    &typed_data,
                    ron::ser::PrettyConfig::default().struct_names(true)
                )
                .unwrap()
            );
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets the directory of the fuzzing corpus
    corpus: PathBuf,
}
