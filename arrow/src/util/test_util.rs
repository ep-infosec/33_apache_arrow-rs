// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Utils to make testing easier

use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{env, error::Error, fs, io::Write, path::PathBuf};

/// Returns a vector of size `n`, filled with randomly generated bytes.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut result = vec![];
    let mut rng = seedable_rng();
    for _ in 0..n {
        result.push(rng.gen_range(0..255));
    }
    result
}

/// Returns fixed seedable RNG
pub fn seedable_rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

/// Returns file handle for a temp file in 'target' directory with a provided content
///
/// TODO: Originates from `parquet` utils, can be merged in [ARROW-4064]
pub fn get_temp_file(file_name: &str, content: &[u8]) -> fs::File {
    // build tmp path to a file in "target/debug/testdata"
    let mut path_buf = env::current_dir().unwrap();
    path_buf.push("target");
    path_buf.push("debug");
    path_buf.push("testdata");
    fs::create_dir_all(&path_buf).unwrap();
    path_buf.push(file_name);

    // write file content
    let mut tmp_file = fs::File::create(path_buf.as_path()).unwrap();
    tmp_file.write_all(content).unwrap();
    tmp_file.sync_all().unwrap();

    // return file handle for both read and write
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path_buf.as_path());
    assert!(file.is_ok());
    file.unwrap()
}

/// Returns the arrow test data directory, which is by default stored
/// in a git submodule rooted at `arrow/testing/data`.
///
/// The default can be overridden by the optional environment
/// variable `ARROW_TEST_DATA`
///
/// panics when the directory can not be found.
///
/// Example:
/// ```
/// let testdata = arrow::util::test_util::arrow_test_data();
/// let csvdata = format!("{}/csv/aggregate_test_100.csv", testdata);
/// assert!(std::path::PathBuf::from(csvdata).exists());
/// ```
pub fn arrow_test_data() -> String {
    match get_data_dir("ARROW_TEST_DATA", "../testing/data") {
        Ok(pb) => pb.display().to_string(),
        Err(err) => panic!("failed to get arrow data dir: {}", err),
    }
}

/// Returns the parquest test data directory, which is by default
/// stored in a git submodule rooted at
/// `arrow/parquest-testing/data`.
///
/// The default can be overridden by the optional environment variable
/// `PARQUET_TEST_DATA`
///
/// panics when the directory can not be found.
///
/// Example:
/// ```
/// let testdata = arrow::util::test_util::parquet_test_data();
/// let filename = format!("{}/binary.parquet", testdata);
/// assert!(std::path::PathBuf::from(filename).exists());
/// ```
pub fn parquet_test_data() -> String {
    match get_data_dir("PARQUET_TEST_DATA", "../parquet-testing/data") {
        Ok(pb) => pb.display().to_string(),
        Err(err) => panic!("failed to get parquet data dir: {}", err),
    }
}

/// Returns a directory path for finding test data.
///
/// udf_env: name of an environment variable
///
/// submodule_dir: fallback path (relative to CARGO_MANIFEST_DIR)
///
///  Returns either:
/// The path referred to in `udf_env` if that variable is set and refers to a directory
/// The submodule_data directory relative to CARGO_MANIFEST_PATH
fn get_data_dir(udf_env: &str, submodule_data: &str) -> Result<PathBuf, Box<dyn Error>> {
    // Try user defined env.
    if let Ok(dir) = env::var(udf_env) {
        let trimmed = dir.trim().to_string();
        if !trimmed.is_empty() {
            let pb = PathBuf::from(trimmed);
            if pb.is_dir() {
                return Ok(pb);
            } else {
                return Err(format!(
                    "the data dir `{}` defined by env {} not found",
                    pb.display(),
                    udf_env
                )
                .into());
            }
        }
    }

    // The env is undefined or its value is trimmed to empty, let's try default dir.

    // env "CARGO_MANIFEST_DIR" is "the directory containing the manifest of your package",
    // set by `cargo run` or `cargo test`, see:
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html
    let dir = env!("CARGO_MANIFEST_DIR");

    let pb = PathBuf::from(dir).join(submodule_data);
    if pb.is_dir() {
        Ok(pb)
    } else {
        Err(format!(
            "env `{}` is undefined or has empty value, and the pre-defined data dir `{}` not found\n\
             HINT: try running `git submodule update --init`",
            udf_env,
            pb.display(),
        ).into())
    }
}

/// An iterator that is untruthful about its actual length
#[derive(Debug, Clone)]
pub struct BadIterator<T> {
    /// where the iterator currently is
    cur: usize,
    /// How many items will this iterator *actually* make
    limit: usize,
    /// How many items this iterator claims it will make
    claimed: usize,
    /// The items to return. If there are fewer items than `limit`
    /// they will be repeated
    pub items: Vec<T>,
}

impl<T> BadIterator<T> {
    /// Create a new iterator for `<limit>` items, but that reports to
    /// produce `<claimed>` items. Must provide at least 1 item.
    pub fn new(limit: usize, claimed: usize, items: Vec<T>) -> Self {
        assert!(!items.is_empty());
        Self {
            cur: 0,
            limit,
            claimed,
            items,
        }
    }
}

impl<T: Clone> Iterator for BadIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.limit {
            let next_item_idx = self.cur % self.items.len();
            let next_item = self.items[next_item_idx].clone();
            self.cur += 1;
            Some(next_item)
        } else {
            None
        }
    }

    /// report whatever the iterator says to
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.claimed as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_data_dir() {
        let udf_env = "get_data_dir";
        let cwd = env::current_dir().unwrap();

        let existing_pb = cwd.join("..");
        let existing = existing_pb.display().to_string();
        let existing_str = existing.as_str();

        let non_existing = cwd.join("non-existing-dir").display().to_string();
        let non_existing_str = non_existing.as_str();

        env::set_var(udf_env, non_existing_str);
        let res = get_data_dir(udf_env, existing_str);
        assert!(res.is_err());

        env::set_var(udf_env, "");
        let res = get_data_dir(udf_env, existing_str);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), existing_pb);

        env::set_var(udf_env, " ");
        let res = get_data_dir(udf_env, existing_str);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), existing_pb);

        env::set_var(udf_env, existing_str);
        let res = get_data_dir(udf_env, existing_str);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), existing_pb);

        env::remove_var(udf_env);
        let res = get_data_dir(udf_env, non_existing_str);
        assert!(res.is_err());

        let res = get_data_dir(udf_env, existing_str);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), existing_pb);
    }

    #[test]
    fn test_happy() {
        let res = arrow_test_data();
        assert!(PathBuf::from(res).is_dir());

        let res = parquet_test_data();
        assert!(PathBuf::from(res).is_dir());
    }
}
