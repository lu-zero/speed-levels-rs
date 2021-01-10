# speed-levels benchmarking

This program relies on hyperfine, install it with `cargo install hyperfine`.

```
USAGE:
    speed-levels-rs [OPTIONS] <INPUT>... --encoders <encoders>... --tag <tag>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --encoders <encoders>...    Specify the encoder paths
    -l, --limit <limit>             Number of frames to encode [default: 10]
    -o, --outdir <outdir>           Output directory [default: ~/Encoded]
    -t, --tag <tag>                 Descriptive tag

ARGS:
    <INPUT>...    Input Files
```
