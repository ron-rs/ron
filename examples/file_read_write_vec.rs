/// Getting RON with type derives and reading/writing a Vec<T> to/from a file.

use ron::{Error, error::SpannedResult};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
};

#[derive(Debug, Deserialize, Serialize)]
struct MetaData {
    created_at: String,
    author: String,
}

#[derive(Debug, Deserialize, Serialize)]
enum UserRole {
    User,
    Admin { key: usize },
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
    name: String,
    email: String,
    comment: String,
    role: UserRole,
    meta: MetaData,
}

fn create_records() -> Vec<User> {
    vec![
        User {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            comment: "New\nLine, and \"quotes\"".into(),
            role: UserRole::Admin { key: 0xDEADFEED },
            meta: MetaData {
                created_at: "2025-06-22".into(),
                author: "Admin".to_string(),
            },
        },
        User {
            name: "Bob".into(),
            email: "bob@example.com".into(),
            comment: "Tabs\ttoo".into(),
            role: UserRole::User,
            meta: MetaData {
                created_at: "2025-06-22".into(),
                author: "Admin".to_string(),
            },
        },
    ]
}

// A basic text file format with individual records separated by a magic separator
fn write_ron_vec_to_str<T: Serialize>(records: &[T]) -> Result<String, Error> {
    let mut mut_str = String::new();

    let as_strings = {
        records
            .into_iter()
            // .map(|record| ron::ser::to_string(&record))
            .map(|record| ron::ser::to_string_pretty(&record, ron::ser::PrettyConfig::default()))
            .collect::<Result<Vec<_>, _>>()?
    };

    as_strings.into_iter().for_each(|s| {
        mut_str.push_str(&s);
        mut_str.push_str("\n=RON_MGC=\n");
    });

    Ok(mut_str)
}

fn write_ron_vec_to_file<T: Serialize>(path: &PathBuf, records: &[T]) -> Result<usize, Error> {
    let mut file = File::create(path)?;

    file.write(write_ron_vec_to_str(records)?.as_bytes())
        .map_err(|err| Error::Io(err.to_string()))
}

fn read_ron_vec_from_str<T: DeserializeOwned>(s: &str) -> SpannedResult<Vec<T>> {
    s //_
        .split("\n=RON_MGC=\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| ron::from_str::<T>(s))
        .collect::<Result<Vec<_>, _>>()
}

fn read_ron_vec_from_file<T: DeserializeOwned>(path: &PathBuf) -> SpannedResult<Vec<T>> {
    let mut file = File::open(path)?;
    let mut content = String::new();

    file.read_to_string(&mut content)?;

    read_ron_vec_from_str(&content)
}

pub fn main() {
    let users = create_records();

    let path = PathBuf::from_str("example.ron").unwrap();

    write_ron_vec_to_file(&path, &users).unwrap();

    let read_users: Vec<User> = read_ron_vec_from_file(&path).unwrap();
    println!("{:?}", read_users);
}
