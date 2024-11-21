use std::{
    collections::HashMap,
    env::args,
    fs::File,
    io::{BufReader, Read},
    path::Path,
    process::Command,
};

use data_encoding::HEXLOWER;
use flate2::{write::GzEncoder, Compression};
use log::debug;
use sha2::{Digest, Sha256};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let result = do_mutate();
    match result {
        Err(err) => {
            println!("Aborting: Error, {err}");
            debug!("{err:?}")
        }
        Ok(file_maps) => {
            for (k, v) in file_maps {
                println!("url {k}");
                println!("sha256 {v}");
            }
        }
    };
    debug!("Doing Cleanup");
    cleanup().expect("Error in cleanup");
    Ok(())
}

fn do_mutate() -> anyhow::Result<HashMap<String, String>> {
    debug!("cargo clean");
    let out = Command::new("cargo").arg("clean").output()?;
    assert!(out.status.success());
    let args = args().collect::<Vec<_>>();
    let bin_name = args
        .get(1)
        .expect("Binary Name is a compulsory positional arg");
    let targets = args[2..args.len()].iter().collect::<Vec<_>>();
    for target in &targets {
        debug!("cargo build --release --target={target}");
        let out = Command::new("cargo")
            .args(&["build", "--release", format!("--target=${target}").as_str()])
            .output()?;
        assert!(out.status.success());
    }

    debug!("cargo pkgid");
    let out = Command::new("cargo").arg("pkgid").output()?;
    assert!(out.status.success());
    let version = String::from_utf8(out.stdout).expect("cargo pkgid contained non-utf8 text");
    assert!(out.status.success());

    // cargo metadata --no-deps --format-version 1 | jq -r '.packages[].targets[] | select( .kind | map(. == "bin") | any ) | .name'
    debug!("rm -rf gh-tmp");
    let out = Command::new("rm").args(&["-r", "-f", "gh-tmp"]).output()?;
    assert!(out.status.success());
    debug!("mkdir gh-tmp");
    let out = Command::new("mkdir").arg("gh-tmp").output()?;
    assert!(out.status.success());

    let mut file_maps = HashMap::<String, String>::new();

    for target in &targets {
        // ocp-0.1.0-macos-arm.tar.gz
        // project_name-version-target.tar.gz
        debug!("Running tarball code");
        let f_name = format!("gh-tmp/{bin_name}-{version}-{target}.tar.gz");
        let tar_gz = File::create(f_name)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        let tar_f_name = format!("target/{target}/release/{bin_name}");
        tar.append_dir_all("", tar_f_name.as_str())?;
        debug!("Calculating sha256 of {tar_f_name}");
        let hash = sha256_digest(tar_f_name.as_str())?;
        file_maps.insert(tar_f_name, hash);
    }

    Ok(file_maps)
}

fn sha256_digest<T: AsRef<Path>>(path: T) -> anyhow::Result<String> {
    let input = File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    Ok(HEXLOWER.encode(digest.as_ref()))
}

fn cleanup() -> anyhow::Result<()> {
    debug!("rm -rf gh-tmp");
    let out = Command::new("rm").args(&["-r", "-f", "gh-tmp"]).output()?;
    assert!(out.status.success());
    Ok(())
}
