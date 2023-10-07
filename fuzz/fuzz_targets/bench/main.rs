#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![warn(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![allow(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::unreachable)]
#![allow(unsafe_code)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]

use std::{
    collections::{hash_map::DefaultHasher, HashMap, VecDeque},
    fs,
    hash::{BuildHasher, Hasher},
    path::PathBuf,
};

use anyhow::Context;
use criterion::{black_box, Criterion};
use ron::ser::PrettyConfig;

#[path = "lib.rs"]
mod typed_data;

struct SeededHasher {
    seed: u64,
}

impl BuildHasher for SeededHasher {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        let mut hasher = DefaultHasher::new();
        hasher.write_u64(self.seed);
        hasher
    }
}

fn main() -> anyhow::Result<()> {
    let mut criterion = Criterion::default().configure_from_args();

    let seed = std::env::var("RON_FUZZ_BENCH_SEED")?.parse()?;
    let max_cases = std::env::var("RON_FUZZ_BENCH_CASES")?.parse()?;

    let mut cases = HashMap::with_capacity_and_hasher(0, SeededHasher { seed });

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
            let options = ron::Options::default().without_recursion_limit();
            let ty = options
                .to_string_pretty(&typed_data.ty(), PrettyConfig::default().struct_names(true))
                .unwrap();
            let value = options
                .to_string_pretty(
                    &typed_data.value(),
                    PrettyConfig::default()
                        .struct_names(true)
                        .compact_arrays(true)
                        .compact_maps(true),
                )
                .unwrap();
            let pretty = options
                .to_string_pretty(
                    &typed_data.pretty_config(),
                    PrettyConfig::default()
                        .struct_names(true)
                        .compact_structs(true),
                )
                .unwrap();
            let ron = options
                .to_string_pretty(&typed_data, typed_data.pretty_config())
                .unwrap();

            if cases.insert((ty, value, ron), (entry, pretty)).is_some() {
                continue;
            }
        }
    }

    for ((ty, value, ron), (entry, pretty)) in cases.into_iter().take(max_cases) {
        let data = fs::read(&entry).context("could not read corpus entry")?;

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

    criterion.final_summary();

    Ok(())
}
