use std::fs;

use anyhow::Context;
use criterion::{black_box, Criterion};

#[path = "lib.rs"]
mod typed_data;

fn main() -> anyhow::Result<()> {
    let mut criterion = Criterion::default().configure_from_args();

    for entry in fs::read_dir("corpus/arbitrary").context("could not iterate over corpus")? {
        let path = entry.context("invalid corpus entry")?.path();
        let data = fs::read(&path).context("could not read corpus entry")?;

        if let Some(typed_data) = typed_data::roundtrip_arbitrary_typed_ron_or_panic(&data) {
            println!("{:=^80}", "");
            println!(
                "{}",
                ron::ser::to_string_pretty(&typed_data, typed_data.pretty_config()).unwrap()
            );
            println!("{:=^80}", "");

            criterion.bench_function(&format!("{:?}", path), |b| {
                b.iter(|| {
                    black_box(typed_data::roundtrip_arbitrary_typed_ron_or_panic(&data));
                })
            });
        }
    }

    criterion.final_summary();

    Ok(())
}
