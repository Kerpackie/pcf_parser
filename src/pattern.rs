use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::str::SplitN;
use byteorder::ReadBytesExt;
use serde::{Serialize, Deserialize};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PatternFileData {
    pub compiled_flag: bool,
    pub version: String,
    pub source_combo_index: i32,
    pub pclk_source_indices: [i32; 8],
    pub vtime_reqd: [String; 9],
    pub cycle_time: [String; 9],
    pub pulse_time: [String; 9],
    pub clk_sources: Vec<String>,
    pub start_addrs: [i32; 8],
    pub end_addrs: [i32; 8],
    pub loop_counts: [i32; 8],
    pub pattern_file_length: i32,
    pub pattern_data: Vec<Vec<u8>>, // [bit][col]
}

/*impl Default for PatternFileData {
    fn default() -> Self {
        Self {
            compiled_flag: false,
            version: String::new(),
            source_combo_index: 0,
            pclk_source_indices: [0; 8],
            vtime_reqd: Default::default(),
            cycle_time: Default::default(),
            pulse_time: Default::default(),
            clk_sources: std::array::from_fn(|_| String::new()),
            start_addrs: [0; 8],
            end_addrs: [0; 8],
            loop_counts: [0; 8],
            pattern_file_length: 0,
            pattern_data: Vec::new(),
        }
    }
}*/

pub fn parse_pcf_file<P: AsRef<Path>>(filename: P) -> io::Result<PatternFileData> {

    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    // Read a fixed length in as a string.
    fn read_fixed(reader: &mut BufReader<File>, len: usize) -> io::Result<String> {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).trim_end().to_string())
    }

    let compiled: String = read_fixed(&mut reader, 10)?;
    let mut parts: SplitN<char> = compiled.splitn(2, ' ');

    let flag: bool = parts
        .next()
        .unwrap_or("False")
        .to_lowercase()
        .parse()
        .unwrap_or(false);

    let version: String = parts
        .next()
        .unwrap_or("")
        .to_string();

    let source_combo_index: i32 = read_fixed(&mut reader, 10)?
        .parse()
        .unwrap_or(0);

    let mut pclk_source_indices: [i32; 8] = [0; 8];
    for i in 0..8 {
        pclk_source_indices[i] = read_fixed(&mut reader, 10)?
            .parse()
            .unwrap_or(0);
    }

    let mut vtime_reqd: [String; 9] = Default::default();
    vtime_reqd[8] = read_fixed(&mut reader, 10)?;
    for i in 0..8 {
        vtime_reqd[i] = read_fixed(&mut reader, 10)?;
    }

    let mut cycle_time: [String; 9] = Default::default();
    cycle_time[8] = read_fixed(&mut reader, 10)?;
    for i in 0..8 {
        cycle_time[i] = read_fixed(&mut reader, 10)?;
    }

    let mut pulse_time: [String; 9] = Default::default();
    pulse_time[8] = read_fixed(&mut reader, 10)?;
    for i in 0..8 {
        pulse_time[i] = read_fixed(&mut reader, 10)?;
    }

    let mut clk_sources = vec![String::new(); 65];
    for i in 1..=64 {
        clk_sources[i] = read_fixed(&mut reader, 10)?;
    }

    let mut start_addrs: [i32; 8] = [0; 8];
    let mut end_addrs: [i32; 8] = [0; 8];
    let mut loop_counts: [i32; 8] = [0; 8];

    for i in 0..8 {
        start_addrs[i] = read_fixed(&mut reader, 10)?
            .parse()
            .unwrap_or(0);

        end_addrs[i] = read_fixed(&mut reader, 10)?
            .parse()
            .unwrap_or(0);

        loop_counts[i] = read_fixed(&mut reader, 10)?
            .parse()
            .unwrap_or(0);
    }

    let pattern_file_length = read_fixed(&mut reader, 10)?
        .parse()
        .unwrap_or(0);
    let cols: usize = (pattern_file_length + 20) as usize;

    let mut pattern_data: Vec<Vec<u8>> = vec![vec![0u8; cols]; 18];

    for col in 0..cols {
        for bit in 0..18 {
            pattern_data[bit][col] = reader.read_u8()?;
        }
    }

    Ok(PatternFileData{
        compiled_flag: flag,
        version,
        source_combo_index,
        pclk_source_indices,
        vtime_reqd,
        cycle_time,
        pulse_time,
        clk_sources,
        start_addrs,
        end_addrs,
        loop_counts,
        pattern_file_length,
        pattern_data,
    })
}

pub fn write_pcf_file<P: AsRef<Path>>(filename: P, data: &PatternFileData) -> io::Result<()> {
    let file: File = File::create(filename)?;
    let mut writer: BufWriter<File> = BufWriter::new(file);

    fn write_fixed(writer: &mut BufWriter<File>, val: &str, len: usize) -> io::Result<()> {
        let mut bytes: Vec<u8> = val
            .as_bytes()
            .to_vec();

        bytes.resize(len, b' ');

        writer.write_all(&bytes[..len])
    }

    let flag_str = if data.compiled_flag { "True" } else { "False" };
    let header   = format!("{} {}", flag_str, data.version);
    write_fixed(&mut writer, &header, 10)?;

    write_fixed(&mut writer, &data.source_combo_index.to_string(), 10)?;

    for v in &data.pclk_source_indices {
        write_fixed(&mut writer, &v.to_string(), 10)?;
    }

    write_fixed(&mut writer, &data.vtime_reqd[8], 10)?;
    for i in 0..8 {
        write_fixed(&mut writer, &data.vtime_reqd[i], 10)?;
    }

    write_fixed(&mut writer, &data.cycle_time[8], 10)?;
    for i in 0..8 {
        write_fixed(&mut writer, &data.cycle_time[i], 10)?;
    }

    write_fixed(&mut writer, &data.pulse_time[8], 10)?;
    for i in 0..8 {
        write_fixed(&mut writer, &data.pulse_time[i], 10)?;
    }

    assert_eq!(data.clk_sources.len(), 65, "clk_sources must have 65 entries");
    for i in 1..=64 {
        write_fixed(&mut writer, &data.clk_sources[i], 10)?;
    }

    for i in 0..8 {
        write_fixed(&mut writer, &data.start_addrs[i].to_string(), 10)?;
        write_fixed(&mut writer, &data.end_addrs[i].to_string(), 10)?;
        write_fixed(&mut writer, &data.loop_counts[i].to_string(), 10)?;
    }

    write_fixed(&mut writer, &data.pattern_file_length.to_string(), 10)?;

    let cols: usize = (data.pattern_file_length + 20) as usize;

    for col in 0..cols {
        for bit in 0..18 {
            writer.write_all(&[data.pattern_data[bit][col]])?;
        }
    }

    writer.flush()?;
    Ok(())

}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests for PatternFileData parsing, writing, and JSON round-trip.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use serde_json;

    /// Build a sample PatternFileData with non-trivial content.
    fn sample_pattern_data() -> PatternFileData {
        let mut data = PatternFileData {
            compiled_flag: true,
            version: "v1.2".into(),
            source_combo_index: 3,
            pclk_source_indices: [1,2,3,4,5,6,7,8],
            vtime_reqd: Default::default(),
            cycle_time: Default::default(),
            pulse_time: Default::default(),
            clk_sources: vec![String::new(); 65],
            start_addrs: [10;8],
            end_addrs: [20;8],
            loop_counts: [2;8],
            pattern_file_length: 5,
            pattern_data: vec![vec![0u8; 25]; 18],
        };

        // fill textual arrays
        for i in 0..9 {
            data.vtime_reqd[i] = format!("VT{}", i);
            data.cycle_time[i] = format!("CT{}", i);
            data.pulse_time[i] = format!("PT{}", i);
        }

        // fill clk_sources 1..=64
        for i in 1..=64 {
            data.clk_sources[i] = format!("CLK{:02}", i);
        }

        // fill pattern_data with varying bytes
        for bit in 0..18 {
            for col in 0..25 {
                data.pattern_data[bit][col] = ((bit + col) % 256) as u8;
            }
        }

        data
    }

    #[test]
    fn round_trip_parse_write() {
        let original = sample_pattern_data();
        let tmp = NamedTempFile::new().unwrap();
        // write to PCF
        write_pcf_file(tmp.path(), &original).expect("write failed");
        // read back
        let parsed = parse_pcf_file(tmp.path()).expect("parse failed");
        assert_eq!(original, parsed, "original vs parsed mismatch");
    }

    #[test]
    fn json_round_trip() {
        let original = sample_pattern_data();
        // Serialize to JSON
        let json = serde_json::to_string_pretty(&original).unwrap();
        // Deserialize back
        let parsed: PatternFileData = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed, "JSON round-trip mismatch");
    }
}
