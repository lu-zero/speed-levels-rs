# speed-levels benchmarking

This program relies on hyperfine, install it with `cargo install hyperfine`.

It tries to encode a number of samples using a 4x4 tiles fixed setting using all the available encoder speed presets.
It is possible to use `taskset` or `numactl` as `RUNNER_COMMAND` to restrict further the number of cores used by the encoder.

## Supported encoders

- [aom](https://aomedia.googlesource.com/aom/)
- [rav1e](https://github.com/xiph/rav1e)
- [svt-av1](https://github.com/AOMediaCodec/SVT-AV1)


## Usage
```
USAGE:
    speed-levels-rs [FLAGS] [OPTIONS] <INPUT>... --encoders <encoders>...

FLAGS:
    -h, --help           Prints help information
        --show-output    Print the stdout and stderr of the benchmark instead of suppressing it. This will increase the
                         time it takes for benchmarks to run, so it should only be used for debugging purposes or when
                         trying to benchmark output speed
    -V, --version        Prints version information

OPTIONS:
    -e, --encoders <encoders>...       Specify the encoder paths
        --extra-aom <extra-aom>        Extra command for the aom instances [env: EXTRA_AOM=]  [default: ]
        --extra-rav1e <extra-rav1e>    Extra command for the rav1e instances [env: EXTRA_RAV1E=]  [default: ]
        --extra-svt <extra-svt>        Extra command for the svt-av1 instances [env: EXTRA_SVT=]  [default: ]
    -l, --limit <limit>                Number of frames to encode [default: 10]
    -O, --outdir <outdir>              Output directory for the encoded files [default: ~/Encoded]
    -o, --outname <outname>            Filename of the aggregate spreadsheet
        --runner <runner>              Use the provided runner to execute the encoder [env: RUNNER_COMMAND=]  [default:
                                       ]
    -r, --runs <runs>                  Perform exactly NUM runs for each command [default: 2]
    -t, --tag <tag>                    Descriptive tag [default: enyo.local-x86_64]
        --threads <threads>            Set the threadpool size [default: 16]

ARGS:
    <INPUT>...    Input Files
```

## TODO

- [x] Spreadsheet aggregation
- [ ] Standard summary of the run
- [ ] Graphs in the summary
- [ ] av-metrics integration

