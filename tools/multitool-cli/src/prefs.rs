use clap::{arg, Args, Subcommand};
use epic_prefs::PlayerPrefsData;
use std::path::PathBuf;
use crate::DataFormat;

#[derive(Args, Clone)]
#[command(version, about, long_about = Some("Encode or decode a player prefs file (XML or raw protobuf)"), aliases = ["p", "pref", "preferences"]
)]
pub(super) struct PrefsArgs {
    #[arg(help = "Location of the player prefs file (XML or raw protobuf)")]
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
#[command(version, about, long_about = Some("Decode a player prefs file (auto-detects XML or raw protobuf)"), aliases = ["d", "unpack", "export"]
)]
pub(super) struct PrefsDecodeArgs {
    #[arg(help = "Data format to output the player prefs in", long, short = 'O', default_value_t=DataFormat::Ron)]
    pub output_as: DataFormat,
}

#[derive(Args, Clone)]
#[command(version, about, long_about = Some("Encode a player prefs file (auto-detects XML or raw protobuf)"), aliases = ["e", "pack", "import", "reimport"]
)]
pub(super) struct PrefsEncodeArgs {
    #[arg(
        help = "Location to save the encoded player prefs file",
        value_name = "FILE"
    )]
    pub output_prefs_path: PathBuf,
}

/// Detect if input file is Android XML format or raw protobuf
fn is_xml_format(data: &[u8]) -> bool {
    let s = String::from_utf8_lossy(data);
    let trimmed = s.trim_start();
    trimmed.starts_with("<?xml") || trimmed.starts_with("<map")
}

pub(super) fn encode_prefs(prefs_args: PrefsArgs, args: PrefsEncodeArgs) -> anyhow::Result<()> {
    let json_file = std::fs::read_to_string(&prefs_args.player_data_file)?;
    let raw_input = std::fs::read(&prefs_args.player_prefs_path)?;
    let is_xml = is_xml_format(&raw_input);

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
    } else {
        // Raw protobuf format (PC save): encode directly to bytes
        let bytes = prefs.to_proto_bytes()?;
        std::fs::write(args.output_prefs_path, &bytes)?;
    }

    Ok(())
}

pub(super) fn decode_prefs(prefs_args: PrefsArgs, args: PrefsDecodeArgs) -> anyhow::Result<()> {
    let raw_input = std::fs::read(&prefs_args.player_prefs_path)?;
    let is_xml = is_xml_format(&raw_input);

    let prefs = if is_xml {
        // Android XML format
        let xml_str = String::from_utf8(raw_input)?;
        PlayerPrefsData::from_prefs_xml(xml_str.as_str())?
    } else {
        // PC save
        PlayerPrefsData::from_proto_bytes(&raw_input)?
    };

    let data = match args.output_as {
        DataFormat::Ron => prefs.to_ron_pretty()?,
        DataFormat::Json => prefs.to_json_pretty()?
    };

    std::fs::write(&prefs_args.player_data_file, data.as_bytes()).map_err(anyhow::Error::new)
}
