use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use clap::Parser;
use icu_locid::locale;
use platform_info::*;
use regex::{Regex, RegexBuilder};
use spreadsheet_ods::{Sheet, Value, WorkBook};

fn default_tag() -> String {
    let pi = PlatformInfo::new().unwrap();

    format!(
        "{}-{}",
        pi.nodename().to_string_lossy(),
        pi.machine().to_string_lossy()
    )
}

#[derive(Debug)]
enum EncoderVersion {
    Aom(String),
    Rav1e(String),
    Svt(String),
}

#[derive(Debug, Parser)]
struct Opt {
    /// Input Files
    #[arg(name = "INPUT", required(true))]
    infiles: Vec<PathBuf>,
    /// Number of frames to encode
    #[arg(long, short, default_value = "10")]
    limit: usize,
    /// Output directory for the encoded files
    #[arg(long, short = 'O', default_value = "~/Encoded")]
    outdir: PathBuf,
    /// Specify the encoder paths
    #[arg(long, short, required(true))]
    encoders: Vec<PathBuf>,
    /// Descriptive tag
    #[arg(long, short, default_value_t = default_tag())]
    tag: String,
    /// Print the stdout and stderr of the benchmark instead of suppressing it. This
    /// will increase the time it takes for benchmarks to run, so it should only be
    /// used for debugging purposes or when trying to benchmark output speed.
    #[arg(long)]
    show_output: bool,
    /// Perform exactly NUM runs for each command.
    #[arg(long, short, default_value = "2")]
    runs: String,
    /// Filename of the aggregate spreadsheet
    #[arg(long, short = 'o')]
    outname: Option<PathBuf>,
    /// Set the threadpool size
    #[arg(long, default_value = "16")]
    threads: usize,
    /// Extra command for the aom instances
    #[arg(long, default_value = "", env = "EXTRA_AOM")]
    extra_aom: String,
    /// Extra command for the rav1e instances
    #[arg(long, default_value = "", env = "EXTRA_RAV1E")]
    extra_rav1e: String,
    /// Extra command for the svt-av1 instances
    #[arg(long, default_value = "", env = "EXTRA_SVT")]
    extra_svt: String,
    /// Use the provided runner to execute the encoder
    #[arg(long, default_value = "", env = "RUNNER_COMMAND")]
    runner: String,
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

fn rav1e_y_option<P: AsRef<Path>>(enc: P) -> bool {
    let out = Command::new(enc.as_ref())
        .arg("--help")
        .output()
        .expect("cannot run the encoder");

    std::str::from_utf8(&out.stdout)
        .ok()
        .and_then(|out| {
            RegexBuilder::new(r"\s*-y")
                .multi_line(true)
                .build()
                .ok()
                .and_then(|re| {
                    let v = re.is_match(out);
                    Some(v)
                })
        })
        .unwrap_or(false)
}

fn rav1e_version<P: AsRef<OsStr>>(enc: P) -> Option<EncoderVersion> {
    let out = Command::new(enc)
        .arg("--version")
        .output()
        .expect("cannot run the encoder");

    std::str::from_utf8(&out.stdout).ok().and_then(|out| {
        Regex::new(r"rav1e (\S+) \((\S+)\)").ok().and_then(|re| {
            re.captures(out)
                .and_then(|caps| {
                    let nominal = caps.get(1);
                    let specific = caps.get(2);
                    if let (Some(nominal), Some(specific)) = (nominal, specific) {
                        Some(if specific.as_str() == "UNKNOWN" {
                            nominal
                        } else {
                            specific
                        })
                    } else {
                        nominal
                    }
                })
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
    fn hyperfine(&self, cmd: &str, levels: (&str, &str), out_name: String) -> Result<Sheet> {
        let mut hf = Command::new("hyperfine");

        hf.arg("-r").arg(&self.runs);
        if self.show_output {
            hf.arg("--show-output");
        }
        let csv_export = format!("{}.csv", out_name);
        let json_export = format!("{}.json", out_name);
        let md_export = format!("{}.md", out_name);

        let child = hf
            .args(&["-P", "ss", levels.0, levels.1])
            .arg(cmd)
            .arg("--export-json")
            .arg(&json_export)
            .arg("--export-csv")
            .arg(&csv_export)
            .arg("--export-markdown")
            .arg(&md_export);

        let mut child = child.spawn().expect("hyperfine failed");

        //        std::io::stdout().write_all(&output.stdout).unwrap();
        //        std::io::stderr().write_all(&output.stderr).unwrap();
        child.wait().expect("failed to wait on hyperfine");

        let mut s = Sheet::new(&out_name);
        let f = File::open(&csv_export)?;
        // Save the header as well.
        let mut r = csv::ReaderBuilder::new().has_headers(false).from_reader(f);
        for (x, res) in r.records().enumerate() {
            let record = res?;
            for (y, cell) in record.iter().enumerate() {
                let val = if let Ok(v) = cell.parse::<f64>() {
                    Value::from(v)
                } else {
                    Value::from(cell)
                };
                s.set_value(x as u32, y as u32, val)
            }
        }

        Ok(s)
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

    fn aom_command<P: AsRef<Path>>(&self, enc: P, infile: P, ver: &str) -> Result<Sheet> {
        let (outfile, stats_file) = self.outfiles(&infile, ver, "aom");

        let run = format!("{} {} --tile-rows=2 --tile-columns=2 --cpu-used={{ss}} --threads={} --limit={} -o {} {} {}",
            self.runner,
            enc.as_ref().display(),
            self.threads,
            self.limit,
            outfile.display(),
            infile.as_ref().display(),
            &self.extra_aom,
        );

        self.hyperfine(&run, ("0", "8"), stats_file)
    }

    fn rav1e_command<P: AsRef<Path>>(&self, enc: P, infile: P, ver: &str) -> Result<Sheet> {
        let (outfile, stats_file) = self.outfiles(&infile, ver, "rav1e");

        let overwrite = if rav1e_y_option(&enc) { "-y" } else { "" };

        let run = format!(
            "{} {} --tiles 16 --threads {} -l {} -s {{ss}} -o {} {} {} {}",
            self.runner,
            enc.as_ref().display(),
            self.threads,
            self.limit,
            outfile.display(),
            infile.as_ref().display(),
            overwrite,
            &self.extra_rav1e,
        );

        self.hyperfine(&run, ("0", "10"), stats_file)
    }
    fn svt_command<P: AsRef<Path>>(&self, enc: P, infile: P, ver: &str) -> Result<Sheet> {
        let (outfile, stats_file) = self.outfiles(&infile, ver, "svt");

        let run = format!(
            "{} {} --preset {{ss}} --tile-rows 2 --tile-columns 2 --lp {} -n {} -b {} -i {} {}",
            self.runner,
            enc.as_ref().display(),
            self.threads,
            self.limit,
            outfile.display(),
            infile.as_ref().display(),
            &self.extra_svt,
        );

        self.hyperfine(&run, ("0", "8"), stats_file)
    }
}

fn main() -> Result<()> {
    let opt = Opt::parse();

    let outdir = if opt.outdir == Path::new("~/Encoded") {
        let outdir = dirs_next::home_dir().expect("Cannot find $HOME");

        outdir.join("Encoded")
    } else {
        opt.outdir.clone()
    };

    std::fs::create_dir_all(outdir)?;

    let mut wb = WorkBook::new(locale!("en_US"));
    for input in &opt.infiles {
        for enc in &opt.encoders {
            use self::EncoderVersion::*;
            let s = match probe_version(enc).expect("Cannot probe the encoder") {
                Aom(ver) => opt.aom_command(enc, input, &ver)?,
                Rav1e(ver) => opt.rav1e_command(enc, input, &ver)?,
                Svt(ver) => opt.svt_command(enc, input, &ver)?,
            };
            wb.push_sheet(s);
        }
    }

    if let Some(outname) = opt.outname {
        spreadsheet_ods::write_ods(&mut wb, outname)?;
    }

    Ok(())
}
