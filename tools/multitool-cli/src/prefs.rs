use clap::{arg, Args, Subcommand};
use epic_prefs::PlayerPrefsData;
use std::path::PathBuf;
use crate::DataFormat;

#[derive(Args, Clone)]
#[command(version, about, long_about = Some("Encode or decode a player prefs file (XML, base64, or raw protobuf)"), aliases = ["p", "pref", "preferences"]
)]
pub(super) struct PrefsArgs {
    #[arg(help = "Location of the player prefs file (XML, base64 protobuf, or raw protobuf)")]
    pub player_prefs_path: PathBuf,
    #[arg(help = "Path to the player data file to be encoded/decoded")]
    pub player_data_file: PathBuf,

    #[command(subcommand)]
    pub prefs_action: PrefsAction,
}

#[derive(Subcommand, Clone)]
pub(super) enum PrefsAction {
    Decode(PrefsDecodeArgs),
    Encode(PrefsEncodeArgs),
}

#[derive(Args, Clone)]
#[command(version, about, long_about = Some("Decode a player prefs file (auto-detects format)"), aliases = ["d", "unpack", "export"]
)]
pub(super) struct PrefsDecodeArgs {
    #[arg(help = "Data format to output the player prefs in", long, short = 'O', default_value_t=DataFormat::Ron)]
    pub output_as: DataFormat,
}

#[derive(Args, Clone)]
#[command(version, about, long_about = Some("Encode a player prefs file (auto-detects format)"), aliases = ["e", "pack", "import", "reimport"]
)]
pub(super) struct PrefsEncodeArgs {
    #[arg(
        help = "Location to save the encoded player prefs file",
        value_name = "FILE"
    )]
    pub output_prefs_path: PathBuf,
}

/// Detect if input file is Android XML format or raw/base64 protobuf
fn is_xml_format(data: &[u8]) -> bool {
    let s = String::from_utf8_lossy(data);
    let trimmed = s.trim_start();
    trimmed.starts_with("<?xml") || trimmed.starts_with("<map")
}

/// Detect if input data looks like base64 text (for encoding output format matching)
fn is_base64_text(data: &[u8]) -> bool {
    // If it's valid UTF-8 and mostly alphanumeric + /+=, it's probably base64
    if let Ok(s) = std::str::from_utf8(data) {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return false;
        }
        trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c.is_ascii_whitespace())
    } else {
        false
    }
}

pub(super) fn decode_prefs(prefs_args: PrefsArgs, args: PrefsDecodeArgs) -> anyhow::Result<()> {
    let raw_input = std::fs::read(&prefs_args.player_prefs_path)?;

    let prefs = if is_xml_format(&raw_input) {
        // Android XML format
        let xml_str = String::from_utf8(raw_input)?;
        PlayerPrefsData::from_prefs_xml(xml_str.as_str())?
    } else {
        // Raw or base64 protobuf (PC save) — from_proto_bytes handles both
        PlayerPrefsData::from_proto_bytes(&raw_input)?
    };

    let data = match args.output_as {
        DataFormat::Ron => prefs.to_ron_pretty()?,
        DataFormat::Json => prefs.to_json_pretty()?
    };

    std::fs::write(&prefs_args.player_data_file, data.as_bytes()).map_err(anyhow::Error::new)
}

pub(super) fn encode_prefs(prefs_args: PrefsArgs, args: PrefsEncodeArgs) -> anyhow::Result<()> {
    let json_file = std::fs::read_to_string(&prefs_args.player_data_file)?;
    let raw_input = std::fs::read(&prefs_args.player_prefs_path)?;
    let is_xml = is_xml_format(&raw_input);
    let was_base64 = !is_xml && is_base64_text(&raw_input);

    let data_format = if json_file.starts_with("{") { DataFormat::Json } else { DataFormat::Ron };

    let prefs = match data_format {
        DataFormat::Ron => PlayerPrefsData::from_ron(json_file.as_str())?,
        DataFormat::Json => PlayerPrefsData::from_json(json_file.as_str())?
    };

    if is_xml {
        // Android XML format: reinsert into XML wrapper
        let xml_str = String::from_utf8(raw_input)?;
        let xml_out = prefs.to_prefs_xml(xml_str.as_str(), None)?;
        std::fs::write(args.output_prefs_path, xml_out.as_bytes())?;
    } else if was_base64 {
        // Base64-encoded protobuf (PC save): output as base64
        let bytes = prefs.to_proto_bytes()?;
        std::fs::write(args.output_prefs_path, &bytes)?;
    } else {
        // Raw protobuf: output as raw bytes
        let bytes = prefs.to_proto_bytes_raw()?;
        std::fs::write(args.output_prefs_path, &bytes)?;
    }

    Ok(())
}