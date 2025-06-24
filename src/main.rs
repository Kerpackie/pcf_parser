use crate::pattern::{parse_pcf_file, write_pcf_file};
use crate::utils::{diff_blocks, diff_files, hex_dump_file};

mod utils;
mod pattern;

fn main() -> std::io::Result<()> {
    let input_path = "TEST1.pcf";
    let output_path = "output.pcf";

    let original = parse_pcf_file(input_path)?;
    write_pcf_file(output_path, &original)?;

    let reloaded = parse_pcf_file(output_path)?;

    if original == reloaded {
        println!("Roundtrip success: data matches");
    } else {
        println!("Roundtrip mismatch!");
    }

    let file1 = "TEST1.pcf";
    let file2 = "output.pcf";

    // Replace with actual file paths or command-line arguments
    //hex_dump_file(file1, 16)?;
    //diff_files(file1, file2, 8)?;
    //diff_blocks(file1, file2, 18, 10)?;
    hex_dump_file(file1, 10)?;
    hex_dump_file(file2, 10)?;
    

    Ok(())
}
