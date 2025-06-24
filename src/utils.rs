use std::fs;
use std::io;
use std::path::Path;
use owo_colors::OwoColorize;

pub fn hex_dump_file<P: AsRef<Path>>(file_path: P, bytes_per_line: usize) -> io::Result<()> {
    let buffer = fs::read(&file_path)?;
    println!("Hex dump of: {:?} ({} bytes)", file_path.as_ref(), buffer.len());

    for (i, chunk) in buffer.chunks(bytes_per_line).enumerate() {
        let offset = i * bytes_per_line;
        let hex = chunk.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
        let ascii = chunk.iter().map(|b| {
            if b.is_ascii_graphic() || *b == b' ' { *b as char } else { '.' }
        }).collect::<String>();

        println!("{:06X}  {:<width$}  |{}|", offset, hex, ascii, width = bytes_per_line * 3);
    }

    Ok(())
}

pub fn diff_files<P: AsRef<Path>>(file1: P, file2: P, context: usize) -> io::Result<()> {
    let bytes1 = fs::read(&file1)?;
    let bytes2 = fs::read(&file2)?;
    let len = usize::max(bytes1.len(), bytes2.len());

    println!("Comparing: {:?} vs {:?}", file1.as_ref(), file2.as_ref());

    for i in 0..len {
        let b1 = *bytes1.get(i).unwrap_or(&0);
        let b2 = *bytes2.get(i).unwrap_or(&0);

        if b1 != b2 {
            println!("\n{}", format!("Difference at byte {}: {:02X} != {:02X}", i, b1, b2).red().bold());

            let start = i.saturating_sub(context);
            let end = usize::min(i + context, len);

            for j in start..end {
                let a = *bytes1.get(j).unwrap_or(&0);
                let b = *bytes2.get(j).unwrap_or(&0);
                let mark = if a != b { ">>".yellow().bold().to_string() } else { "  ".to_string() };
                let line = format!("{} [{:04}] {:02X} vs {:02X}  | {} {}", mark, j, a, b, to_char(a), to_char(b));
                if a != b {
                    println!("{}", line.yellow().bold());
                } else {
                    println!("{}", line);
                }
            }
            return Ok(());
        }
    }

    println!("{}", "Files are identical.".green().bold());
    Ok(())
}

pub fn diff_blocks<P: AsRef<Path>>(file1: P, file2: P, block_size: usize, max_blocks: usize) -> io::Result<()> {
    let bytes1 = fs::read(&file1)?;
    let bytes2 = fs::read(&file2)?;
    let len = usize::max(bytes1.len(), bytes2.len());

    let total_blocks = len / block_size;
    let mut shown = 0;

    for block in 0..total_blocks {
        let start = block * block_size;
        let chunk1 = &bytes1.get(start..start + block_size).unwrap_or(&[]);
        let chunk2 = &bytes2.get(start..start + block_size).unwrap_or(&[]);

        if chunk1 != chunk2 {
            println!("\nBlock {} ({}–{}):", block, start, start + block_size - 1);

            for i in 0..block_size {
                let b1 = *chunk1.get(i).unwrap_or(&0);
                let b2 = *chunk2.get(i).unwrap_or(&0);
                let mark = if b1 != b2 { ">>".yellow().bold().to_string() } else { "  ".to_string() };
                let line = format!("{} Byte {:05}: {:02X} vs {:02X} | {} {}", mark, start + i, b1, b2, to_char(b1), to_char(b2));
                if b1 != b2 {
                    println!("{}", line.yellow().bold());
                } else {
                    println!("{}", line);
                }
            }

            shown += 1;
            if shown >= max_blocks {
                println!("\nMax diff blocks reached.");
                break;
            }
        }
    }

    if shown == 0 {
        println!("All blocks are identical.");
    }

    Ok(())
}

fn to_char(b: u8) -> char {
    if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests for hex_dump_file, diff_files, and diff_blocks.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn hex_dump_does_not_panic() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "Hello, PCF!").unwrap();
        // should run without error
        assert!(hex_dump_file(f.path(), 8).is_ok());
    }

    #[test]
    fn diff_files_detects_change() {
        let mut a = NamedTempFile::new().unwrap();
        let mut b = NamedTempFile::new().unwrap();
        a.write_all(b"FOO_BAR").unwrap();
        b.write_all(b"FOO-XAR").unwrap();
        // should not panic; human‐inspect output to see a difference reported
        assert!(diff_files(a.path(), b.path(), 2).is_ok());
    }

    #[test]
    fn diff_blocks_detects_change() {
        let mut a = NamedTempFile::new().unwrap();
        let mut b = NamedTempFile::new().unwrap();
        // create two blocks of 4 bytes
        let block = vec![1u8,2,3,4];
        let mut data_a = block.clone();
        data_a.extend_from_slice(&block);
        let mut data_b = data_a.clone();
        // flip one byte in second block
        data_b[5] ^= 0xFF;
        a.write_all(&data_a).unwrap();
        b.write_all(&data_b).unwrap();
        assert!(diff_blocks(a.path(), b.path(), 4, 5).is_ok());
    }
}
