use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::PathBuf,
};

use anyhow::Context;
use criterion::{black_box, Criterion};
use ron::ser::PrettyConfig;

#[path = "lib.rs"]
mod typed_data;

fn main() -> anyhow::Result<()> {
    let mut criterion = Criterion::default().configure_from_args();

    let mut cases = HashSet::new();

    let mut entries = VecDeque::new();
    entries.push_back(PathBuf::from("corpus/arbitrary"));

    while let Some(entry) = entries.pop_front() {
        if entry.is_dir() {
            for entry in fs::read_dir(entry).context("could not iterate over corpus")? {
                let path = entry.context("invalid corpus entry")?.path();
                entries.push_back(path);
            }
            continue;
        }

        let data = fs::read(&entry).context("could not read corpus entry")?;

        if let Some(typed_data) = typed_data::roundtrip_arbitrary_typed_ron_or_panic(&data) {
            let ty = ron::ser::to_string_pretty(
                &typed_data.ty(),
                PrettyConfig::default().struct_names(true),
            )
            .unwrap();
            let value = ron::ser::to_string_pretty(
                &typed_data.value(),
                PrettyConfig::default()
                    .struct_names(true)
                    .compact_arrays(true)
                    .compact_maps(true),
            )
            .unwrap();
            let pretty = ron::ser::to_string_pretty(
                &typed_data.pretty_config(),
                PrettyConfig::default()
                    .struct_names(true)
                    .compact_structs(true),
            )
            .unwrap();
            let ron = ron::ser::to_string_pretty(&typed_data, typed_data.pretty_config()).unwrap();

            if !cases.insert((ty.clone(), value.clone(), ron.clone())) {
                continue;
            }

            println!("{:=^80}", " benchmark case ");
            println!("{:^80}", entry.to_string_lossy());
            println!("{:=^80}", " type ");
            println!("{ty}");
            println!("{:=^80}", " value ");
            println!("{value}");
            println!("{:=^80}", " pretty config ");
            println!("{pretty}");
            println!("{:=^80}", " pretty ron ");
            println!("{ron}");
            println!("{:=^80}", "");

            criterion.bench_function(&format!("{:?}", entry), |b| {
                b.iter(|| {
                    black_box(typed_data::roundtrip_arbitrary_typed_ron_or_panic(&data));
                })
            });
        }
    }

    criterion.final_summary();

    Ok(())
}
