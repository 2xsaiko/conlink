# conlink

Launches a command and provides its output/input over a TCP socket

Connecting to the socket tested with netcat or ncat (part of [nmap](https://nmap.org/download.html)).

    USAGE:
        conlink [FLAGS] [OPTIONS] [--] <command>...
    
    FLAGS:
        -b, --binary     Enable binary mode
        -e, --echo       Send input from client to other clients
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