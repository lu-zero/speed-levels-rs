# speed-levels benchmarking

This program relies on hyperfine, install it with `cargo install hyperfine`.

## Supported encoders

- [aom](https://aomedia.googlesource.com/aom/)
- [rav1e](https://github.com/xiph/rav1e)
- [svt-av1](https://github.com/AOMediaCodec/SVT-AV1)

## Usage

The program can be used to test the encoders speed levels. Currently all the speed-levels are tested (`0-8` for `aom` and `svt-av1`, `0-10` for `rav1e`)

```
USAGE:
    speed-levels-rs [FLAGS] [OPTIONS] <INPUT>... --encoders <encoders>... --tag <tag>

FLAGS:
    -h, --help           Prints help information
        --show-output    Print the stdout and stderr of the benchmark instead of suppressing it. This will increase the
                         time it takes for benchmarks to run, so it should only be used for debugging purposes or when
                         trying to benchmark output speed
    -V, --version        Prints version information

OPTIONS:
    -e, --encoders <encoders>...    Specify the encoder paths
    -l, --limit <limit>             Number of frames to encode [default: 10]
    -O, --outdir <outdir>           Output directory for the encoded files [default: ~/Encoded]
    -o, --outname <outname>         Filename of the aggregate spreadsheet
    -r, --runs <runs>               Perform exactly NUM runs for each command [default: 2]
    -t, --tag <tag>                 Descriptive tag

ARGS:
    <INPUT>...    Input Files
```

## TODO

- [x] Spreadsheet aggregation
- [ ] Standard summary of the run
- [ ] Graphs in the summary
- [ ] av-metrics integration

