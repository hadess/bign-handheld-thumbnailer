mod main_errors;
mod n3ds;
mod nds;
mod utils;

use gdk_pixbuf::{InterpType, Pixbuf};
use main_errors::*;
use n3ds::n3ds_structures::SMDHIcon;
use nds::extract_nds_banner;
use pico_args::Arguments;
use std::fs::File;
use std::{path::Path, process::ExitCode};

#[derive(Debug, Clone)]
struct ThumbnailerArgs {
    show_version: bool,
    file_params: Option<ThumbnailerArgsFileParams>,
}

impl ThumbnailerArgs {
    pub fn file_params(&self) -> Option<ThumbnailerArgsFileParams> {
        self.file_params.clone()
    }
}

#[derive(Debug, Clone)]
struct ThumbnailerArgsFileParams {
    size: Option<i32>,
    input_file: std::path::PathBuf,
    output_file: std::path::PathBuf,
}

fn main() -> ExitCode {
    let args = Arguments::from_env();

    let args = match get_thumbnailer_args(&args) {
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
        Ok(args) => args,
    };
    if let Err(e) = bign_handheld_thumbnailer(&args) {
        eprintln!("{}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn get_thumbnailer_args(arguments: &Arguments) -> Result<ThumbnailerArgs, MainError> {
    let mut args = arguments.clone();

    let show_version = args.contains("--version");
    let file_params = if !show_version {
        Some(get_thumbnailer_args_file_params(&mut args)?)
    } else {
        None
    };

    Ok(ThumbnailerArgs {
        show_version,
        file_params,
    })
}

fn get_thumbnailer_args_file_params(
    args: &mut Arguments,
) -> Result<ThumbnailerArgsFileParams, MainError> {
    let size = args.opt_value_from_str("-s")?;
    let input_file = args.free_from_str()?;
    let output_file = args.free_from_str()?;

    Ok(ThumbnailerArgsFileParams {
        size,
        input_file,
        output_file,
    })
}

fn bign_handheld_thumbnailer(args: &ThumbnailerArgs) -> Result<(), MainError> {
    if args.show_version {
        const NAME: &str = env!("CARGO_PKG_NAME");
        const VERSION: &str = env!("CARGO_PKG_VERSION");

        println!("{} v{}", NAME, VERSION);

        return Ok(());
    }

    // if it's not a `--version` command, then just extract the file params directly
    let file_params = args.file_params().unwrap();

    let input = Path::new(&file_params.input_file);
    let output = Path::new(&file_params.output_file);
    let size = file_params.size;

    let content_type = utils::content_type_guess(Some(input), None);
    let content_type = content_type.0.as_str();

    /* There are currently two supported file types:
     * .nds roms, indicated by the application/x-nintendo-ds-rom mime type
     * and .cia files, indicated by the application/x-ctr-cia mime type
     *
     * Note that application/x-ctr-cia is the same mime type used by Citra
     * and might not be defined on the user system
     * Therefore .cia support might require shipping .cia mime type support
     */

    // You might want to check https://github.com/citra-emu/citra/blob/master/dist/citra.xml
    // for the Nintendo 3DS-related mime types as defined by the Citra emulator

    let mut input = File::open(input)?;
    let pixbuf = match content_type {
        "application/x-nintendo-ds-rom" => extract_nds_banner(&mut input)?.get_icon(),
        "application/x-ctr-cia" => SMDHIcon::from_cia(&mut input)?.get_large_icon(),
        "application/x-ctr-smdh" => SMDHIcon::from_smdh(&mut input)?.get_large_icon(),
        "application/x-ctr-3dsx" | "application/x-nintendo-3ds-executable" => {
            SMDHIcon::from_n3dsx(&mut input)?.get_large_icon()
        }
        "application/x-ctr-cxi" => SMDHIcon::from_cxi(&mut input)?.get_large_icon(),
        "application/x-ctr-cci" | "application/x-nintendo-3ds-rom" => {
            SMDHIcon::from_cci(&mut input)?.get_large_icon()
        }
        _ => {
            return Err(MainError::InvalidContentType {
                0: content_type.to_string(),
            })
        }
    };

    let pixbuf = scale_pixbuf(pixbuf, size);
    pixbuf.savev(output, "png", &[])?;
    Ok(())
}

fn scale_pixbuf(pixbuf: Pixbuf, size: Option<i32>) -> Pixbuf {
    if let Some(size) = size {
        match pixbuf.scale_simple(size, size, InterpType::Bilinear) {
            Some(scaled_pixbuf) => return scaled_pixbuf,
            None => return pixbuf,
        };
    };

    // Unscaled Pixbuf if size is a None
    pixbuf
}
