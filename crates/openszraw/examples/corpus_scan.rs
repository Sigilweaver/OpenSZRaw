//! Walks the full local `.qgd`/`.lcd` corpus (out of tree, not part of
//! `cargo test`) and runs `assert_source_invariants` against every file,
//! reporting pass/fail counts and per-format breakdowns. Run on demand:
//!
//! ```sh
//! OPENSZRAW_CORPUS=/path/to/SZRaw cargo run --release --example corpus_scan
//! ```
//!
//! `OPENSZRAW_CORPUS` points at the corpus root holding `index.csv` and
//! one directory per accession (see `CORPUS.md`).

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use openmassspec_core::conformance::assert_source_invariants;
use openszraw::reader::Reader;

fn main() {
    let Some(base_dir) = std::env::var_os("OPENSZRAW_CORPUS")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
    else {
        eprintln!("OPENSZRAW_CORPUS is not set - nothing to scan (corpus lives out of tree)");
        return;
    };
    let index_path = base_dir.join("index.csv");

    if !index_path.exists() {
        eprintln!(
            "corpus index not found at {} - nothing to scan",
            index_path.display()
        );
        return;
    }

    let file = File::open(&index_path).expect("failed to open index.csv");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    lines.next(); // header row

    let mut pass_count = 0usize;
    let mut fail_count = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();
    let mut total_spectra = 0usize;
    let mut total_count = 0usize;

    for line_result in lines {
        let line = line_result.expect("failed to read line");
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            continue;
        }
        let accession = parts[1];
        let file_name = parts[2];
        let path = base_dir.join(accession).join(file_name);

        total_count += 1;
        print!("[{total_count}] {accession}/{file_name} ... ");

        match Reader::open(&path) {
            Ok(mut reader) => {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    assert_source_invariants(&mut reader)
                })) {
                    Ok(Ok(n)) => {
                        pass_count += 1;
                        total_spectra += n;
                        println!("ok ({n} spectra)");
                    }
                    Ok(Err(e)) => {
                        fail_count += 1;
                        println!("CONFORMANCE FAIL: {e:?}");
                        failures.push((format!("{accession}/{file_name}"), format!("{e:?}")));
                    }
                    Err(_) => {
                        fail_count += 1;
                        println!("PANIC during conformance check");
                        failures.push((
                            format!("{accession}/{file_name}"),
                            "panic during conformance check".to_string(),
                        ));
                    }
                }
            }
            Err(e) => {
                fail_count += 1;
                println!("OPEN FAIL: {e}");
                failures.push((
                    format!("{accession}/{file_name}"),
                    format!("open error: {e}"),
                ));
            }
        }
    }

    println!("\n=== Corpus Scan Summary ===");
    println!("Total files scanned: {total_count}");
    println!("Passed: {pass_count}");
    println!("Failed: {fail_count}");
    println!("Total spectra decoded: {total_spectra}");

    if !failures.is_empty() {
        println!("\nFailures:");
        for (path, reason) in &failures {
            println!("- {path}: {reason}");
        }
    }
}
