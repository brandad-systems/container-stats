# container-stats

A small tool to analyze RAM usage of large amounts of docker containers.

```
container-stats 0.1.0
MCOfficer <mcofficer@gmx.de>
A small tool to analyze RAM usage of large amounts of docker containers.

USAGE:
    container-stats [FLAGS] [OPTIONS]

FLAGS:
        --group-by-prefix    Group containers by prefix
        --group-by-suffix    Group containers by suffix
    -h, --help               Prints help information
    -s, --sort               Sorts containers by memory used
        --top                Use docker top. Not supported on windows & significantly slower, but correctly detects
                             multiple processes per container
    -t, --total              Prints total memory used by containers
    -V, --version            Prints version information

OPTIONS:
    -d, --delimiter <delimiter>              Delimiter for grouping [default: -]
    -m, --memory-backend <memory-backend>    The way the used memory is calculated. Options are: "procmaps" (cross-
                                             platform), "rss" and "vsz" (both linux) [default: procmaps]
    -r, --regex <regex>                      Filters container names by a regular expression [default: .*]
```

## Building

```
$ cargo build [--release]
```

## Samples

List all containers:
```
$ sudo ./container-stats 
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
|  memory  |  pid   |                           name                           |                                id                                |
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
|  4.4 GB  | 149498 |                    /sample-container-1                   | 2b13f54b6493cb396ae5122a343053a84a2af5aab1c79eb2f5a58b6a9bb234ca |
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
|  4.8 GB  | 60271  |         /some-slightly-longer-sample-container           | 3dd1f8261c053f9ee5908033bd8798a60120e8e0423c74d83bd8dceda22bd157 |
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
|  3.5 GB  | 28350  |                    /sample-container-2                   | 245c06ad91274c40a7f4e0090d5f74112b74818a4d237b5485962f142e74ac1c |
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
|  3.9 GB  | 73631  |      /some-pretty-damn-long-sample-container-name        | 61f2cb9dc16d047f42152dde98c00e5a9f9e2a8ba305adfbf38252af3586e07a |
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
| 17.2 GB  | 195327 |                  /sample-container-three                 | 1b05272735bf083e1b148169130f620505dbc2fcff3c4fc6f8f46e49a4efa676 |
+----------+--------+----------------------------------------------------------+------------------------------------------------------------------+
```

Show the RSS usage of all containers, combined
```
$ sudo ./container-stats -m rss -t
Total: 114.3 GB (114328526848 B)
```
(To learn about the difference between RSS and VSZ, see https://stackoverflow.com/questions/7880784)


The total memory usage of all containers starting with `/sample-`:
```
$ sudo ./container-stats -t -r "^/sample-"
Total: 106.4 GB (106439839744 B)
```

All containers, grouped by their prefixes, sorted by memory usage:
```
$ sudo ./container-stats --group-by-prefix -s
+----------+------------+-------------+
|  memory  | containers |     fix     |
+----------+------------+-------------+
| 239.3 GB |     18     |   /fatfix   |
+----------+------------+-------------+
| 133.1 GB |     10     |  /prefix1   |
+----------+------------+-------------+
| 106.4 GB |     24     |  /prefix2   |
+----------+------------+-------------+
| 88.3 GB  |     7      |    /fix     |
+----------+------------+-------------+
| 35.8 GB  |     10     |   /boring   |
+----------+------------+-------------+
| 26.1 GB  |     6      |   /foofix   |
+----------+------------+-------------+
| 14.7 GB  |     4      |   /barfix   |
+----------+------------+-------------+
| 11.3 GB  |     3      |  /prefix4   |
+----------+------------+-------------+
| 127.9 MB |     1      |    /tiny    |
+----------+------------+-------------+
```