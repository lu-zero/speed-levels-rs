use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use regex::Regex;
use structopt::clap::AppSettings::*;
use structopt::StructOpt;

#[derive(Debug)]
enum EncoderVersion {
    Aom(String),
    Rav1e(String),
    Svt(String),
}

#[derive(Debug, StructOpt)]
#[structopt(global_setting(ColoredHelp))]
struct Opt {
    /// Input Files
    #[structopt(name = "INPUT", required(true))]
    infiles: Vec<PathBuf>,
    /// Number of frames to encode
    #[structopt(long, short, default_value = "10")]
    limit: usize,
    /// Output directory
    #[structopt(long, short, parse(from_os_str), default_value = "~/Encoded")]
    outdir: PathBuf,
    /// Specify the encoder paths
    #[structopt(long, short, required(true))]
    encoders: Vec<PathBuf>,
    /// Descriptive tag
    #[structopt(long, short)]
    tag: String,
}

fn aom_version<P: AsRef<OsStr>>(enc: P) -> Option<EncoderVersion> {
    let out = Command::new(enc)
        .arg("--help")
        .output()
        .expect("cannot run the encoder");

    std::str::from_utf8(&out.stdout).ok().and_then(|out| {
        Regex::new(r"av1    - AOMedia Project AV1 Encoder (\S+) ")
            .ok()
            .and_then(|re| {
                re.captures(out)
                    .and_then(|caps| caps.get(1))
                    .map(|ver| EncoderVersion::Aom(ver.as_str().to_owned()))
            })
    })
}

fn rav1e_version<P: AsRef<OsStr>>(enc: P) -> Option<EncoderVersion> {
    let out = Command::new(enc)
        .arg("--help")
        .output()
        .expect("cannot run the encoder");

    std::str::from_utf8(&out.stdout).ok().and_then(|out| {
        Regex::new(r"rav1e (\S+) ").ok().and_then(|re| {
            re.captures(out)
                .and_then(|caps| caps.get(1))
                .map(|ver| EncoderVersion::Rav1e(ver.as_str().to_owned()))
        })
    })
}

fn svt_version<P: AsRef<OsStr>>(enc: P) -> Option<EncoderVersion> {
    let out = Command::new(enc).output().expect("cannot run the encoder");
    std::str::from_utf8(&out.stderr).ok().and_then(|out| {
        Regex::new(r"SVT \[version\]:	SVT-AV1 Encoder Lib (\S+)\s")
            .ok()
            .and_then(|re| {
                re.captures(out)
                    .and_then(|caps| caps.get(1))
                    .map(|ver| EncoderVersion::Svt(ver.as_str().to_owned()))
            })
    })
}

fn probe_version<P: AsRef<OsStr>>(enc: P) -> Option<EncoderVersion> {
    aom_version(&enc).or_else(|| rav1e_version(&enc).or_else(|| svt_version(&enc)))
}

impl Opt {
    fn hyperfine(&self, cmd: &str, levels: (&str, &str), out_name: String) {
        let mut child = Command::new("hyperfine")
            .args(&["-r", "2"])
            .args(&["-P", "ss", levels.0, levels.1])
            .arg(cmd)
            .arg("--export-csv")
            .arg(&format!("{}.csv", out_name))
            .arg("--export-markdown")
            .arg(&format!("{}.md", out_name))
            .spawn()
            .expect("hyperfine failed");

        //        std::io::stdout().write_all(&output.stdout).unwrap();
        //        std::io::stderr().write_all(&output.stderr).unwrap();
        child.wait().expect("failed to wait on hyperfine");
    }

    fn outfiles<P: AsRef<Path>>(&self, infile: P, ver: &str, kind: &str) -> (PathBuf, String) {
        let name = infile
            .as_ref()
            .file_stem()
            .expect("invalid filename")
            .to_str()
            .unwrap();
        let enc = format!("{}-{}", kind, ver);

        let outfile = self
            .outdir
            .join(format!("{}-{}-{{ss}}-l{}.ivf", name, enc, self.limit));

        let stats_file = format!("{}-{}-speed-levels-{}-l{}", self.tag, enc, name, self.limit);

        (outfile, stats_file)
    }

    fn aom_command<P: AsRef<Path>>(&self, enc: P, infile: P, ver: &str) -> Result<()> {
        println!("{} {}", infile.as_ref().display(), ver);

        let (outfile, stats_file) = self.outfiles(&infile, ver, "aom");

        let runner = std::env::var("RUNNER_COMMAND").unwrap_or_default();

        let run = format!("{} {} --tile-rows=2 --tile-columns=2 --cpu-used={{ss}} --threads=16 --limit={} -o {} {}",
            runner, enc.as_ref().display(), self.limit, outfile.display(), infile.as_ref().display());

        self.hyperfine(&run, ("0", "8"), stats_file);

        Ok(())
    }

    fn rav1e_command<P: AsRef<Path>>(&self, enc: P, infile: P, ver: &str) -> Result<()> {
        let (outfile, stats_file) = self.outfiles(&infile, ver, "rav1e");

        let runner = std::env::var("RUNNER_COMMAND").unwrap_or_default();

        let overwrite = if !ver.starts_with("0.3") { "-y" } else { "" };

        let run = format!(
            "{} {} --threads 16 --tiles 16 -l {} -s {{ss}} -o {} {} {}",
            runner,
            enc.as_ref().display(),
            self.limit,
            outfile.display(),
            infile.as_ref().display(),
            overwrite
        );

        self.hyperfine(&run, ("0", "10"), stats_file);

        Ok(())
    }
    fn svt_command<P: AsRef<Path>>(&self, enc: P, infile: P, ver: &str) -> Result<()> {
        let (outfile, stats_file) = self.outfiles(&infile, ver, "svt");

        let runner = std::env::var("RUNNER_COMMAND").unwrap_or_default();

        let run = format!(
            "{} {} --preset {{ss}} --tile-rows 2 --tile-columns 2 --lp 16 -n {} -b {} -i {}",
            runner,
            enc.as_ref().display(),
            self.limit,
            outfile.display(),
            infile.as_ref().display(),
        );

        self.hyperfine(&run, ("0", "8"), stats_file);

        Ok(())
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    for input in &opt.infiles {
        for enc in &opt.encoders {
            use self::EncoderVersion::*;
            match probe_version(enc).expect("Cannot probe the encoder") {
                Aom(ver) => opt.aom_command(enc, input, &ver)?,
                Rav1e(ver) => opt.rav1e_command(enc, input, &ver)?,
                Svt(ver) => opt.svt_command(enc, input, &ver)?,
            }
        }
    }

    Ok(())
}
