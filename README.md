# conlink

Launches a command and provides its output/input over a TCP socket

netcat or ncat (part of [nmap](https://nmap.org/download.html)) is recommended to connect.

    USAGE:
        conlink [FLAGS] [OPTIONS] [--] <command>...
    
    FLAGS:
        -h, --help       Prints help information
        -q, --quiet      Disable passthrough of command output/input to stdout/stdin
        -V, --version    Prints version information
    
    OPTIONS:
        -H, --host <host>    The host to bind the socket to [default: 0.0.0.0]
        -p, --port <port>    The port to bind the socket to [default: 1337]
    
    ARGS:
        <command>...    The command to run

## Examples

    $ conlink -- /bin/bash
    $ conlink -qp 7100 -- dmesg -w
    $ conlink -qH 127.0.0.1 -p 7100 -- yes