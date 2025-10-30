#!/usr/bin/env rust-script
//! Migrate old vectors.bin format to new format with length prefixes
//!
//! Usage: rust-script migrate_vectors.rs /path/to/collection/dir

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::path::PathBuf;

// Simplified Vector structure for migration
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Vector {
    id: uuid::Uuid,
    data: Vec<f32>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <collection_dir>", args[0]);
        eprintln!("Example: {} /root/embedding-project/dvecdb-data/incidents", args[0]);
        std::process::exit(1);
    }

    let collection_dir = PathBuf::from(&args[1]);
    let old_file = collection_dir.join("vectors.bin");
    let new_file = collection_dir.join("vectors.bin.new");
    let backup_file = collection_dir.join("vectors.bin.backup");

    println!("d-vecDB Vector Format Migration Tool");
    println!("=====================================");
    println!();
    println!("Collection: {}", collection_dir.display());
    println!("Old file: {}", old_file.display());
    println!();

    // Check files exist
    if !old_file.exists() {
        eprintln!("ERROR: vectors.bin not found at {}", old_file.display());
        std::process::exit(1);
    }

    let old_size = fs::metadata(&old_file)?.len();
    println!("Old file size: {} bytes ({:.2} MB)", old_size, old_size as f64 / 1_000_000.0);
    println!();

    // Read old format and convert to new format
    println!("Reading old format...");
    let old_data = fs::read(&old_file)?;
    let mut reader = BufReader::new(&old_data[..]);

    let mut vectors = Vec::new();
    let mut errors = 0;

    // Try to deserialize vectors one by one
    loop {
        match bincode::deserialize_from::<_, Vector>(&mut reader) {
            Ok(vector) => {
                vectors.push(vector);
                if vectors.len() % 1000 == 0 {
                    print!("\rRead {} vectors...", vectors.len());
                    io::stdout().flush()?;
                }
            }
            Err(e) => {
                // Check if we reached end of file
                if reader.get_ref().is_empty() ||
                   format!("{:?}", e).contains("UnexpectedEof") ||
                   format!("{:?}", e).contains("Io") {
                    break; // End of file reached
                }
                errors += 1;
                eprintln!("\nWarning: Deserialization error after {} vectors: {:?}", vectors.len(), e);

                if errors > 10 {
                    eprintln!("Too many errors, stopping");
                    break;
                }
            }
        }
    }

    println!("\r✅ Read {} vectors from old format", vectors.len());

    if vectors.is_empty() {
        eprintln!("\n❌ ERROR: No vectors could be read from the old file!");
        eprintln!("   The file might be in a completely different format.");
        eprintln!("   First 64 bytes of file:");
        let preview = &old_data[..64.min(old_data.len())];
        for chunk in preview.chunks(16) {
            eprint!("   ");
            for byte in chunk {
                eprint!("{:02x} ", byte);
            }
            eprintln!();
        }
        std::process::exit(1);
    }

    // Write new format with length prefixes
    println!("Writing new format with length prefixes...");
    let mut writer = BufWriter::new(File::create(&new_file)?);
    let mut written = 0;

    for (i, vector) in vectors.iter().enumerate() {
        // Serialize vector
        let serialized = bincode::serialize(vector)?;

        // Write length prefix (4 bytes, u32 little-endian)
        let length = serialized.len() as u32;
        writer.write_all(&length.to_le_bytes())?;

        // Write serialized data
        writer.write_all(&serialized)?;

        written += 1;
        if written % 1000 == 0 {
            print!("\rWrote {} vectors...", written);
            io::stdout().flush()?;
        }
    }

    writer.flush()?;
    println!("\r✅ Wrote {} vectors to new format", written);

    let new_size = fs::metadata(&new_file)?.len();
    println!();
    println!("Results:");
    println!("  Old size: {} bytes", old_size);
    println!("  New size: {} bytes", new_size);
    println!("  Overhead: {} bytes ({:.2}%)",
             new_size.saturating_sub(old_size),
             (new_size as f64 - old_size as f64) / old_size as f64 * 100.0);
    println!();

    // Create backup and replace
    println!("Creating backup...");
    fs::copy(&old_file, &backup_file)?;
    println!("✅ Backup created: {}", backup_file.display());

    println!("Replacing old file with new format...");
    fs::rename(&new_file, &old_file)?;
    println!("✅ Migration complete!");
    println!();
    println!("Summary:");
    println!("  ✅ Migrated {} vectors", vectors.len());
    println!("  ✅ Old file backed up to: {}", backup_file.display());
    println!("  ✅ New format file: {}", old_file.display());
    println!();
    println!("Next steps:");
    println!("  1. Restart your d-vecDB server");
    println!("  2. Check logs for: 'Loaded {} vectors from storage'", vectors.len());
    println!("  3. Test search to verify vectors are loaded");
    println!();

    Ok(())
}
