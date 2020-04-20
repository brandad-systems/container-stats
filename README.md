# container-stats

A small tool to analyze RAM usage of large amounts of docker containers.

```
container-stats 0.4.0
MCOfficer <mcofficer@gmx.de>
A small tool to analyze RAM usage of large amounts of docker containers.

USAGE:
    container-stats [FLAGS] [OPTIONS]

FLAGS:
        --group-by-prefix    Group containers by prefix
        --group-by-suffix    Group containers by suffix
    -h, --help               Prints help information
        --json               Print as json instead of a table
    -s, --sort               Sorts containers by memory used
        --top                Use docker top. Not supported on windows & significantly slower, but correctly detects
                             multiple processes per container
    -t, --total              Prints total memory used by containers
    -V, --version            Prints version information

OPTIONS:
        --debug <debug>                      The logging level, in case the RUST_LOG environment variable cannot be set
    -d, --delimiter <delimiter>              Delimiter for grouping [default: -]
    -m, --memory-backend <memory-backend>    The way the used memory is calculated. Options are: "procmaps" (cross-
                                             platform), "rss" and "vsz" (both linux) [default: procmaps]
    -r, --regex <regex>                      Filters container names by a regular expression
```

## Building

```
$ cargo build [--release]
```

## Samples

List all containers:
```
$ sudo ./container-stats 
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
|  memory  | average_percent_cpu |                           name                           |                                id                                |
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
|  4.4 GB  |     0.054178316     |                    /sample-container-1                   | 2b13f54b6493cb396ae5122a343053a84a2af5aab1c79eb2f5a58b6a9bb234ca |
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
|  4.8 GB  |     0.001549211     |         /some-slightly-longer-sample-container           | 3dd1f8261c053f9ee5908033bd8798a60120e8e0423c74d83bd8dceda22bd157 |
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
|  3.5 GB  |     0.705093403     |                    /sample-container-2                   | 245c06ad91274c40a7f4e0090d5f74112b74818a4d237b5485962f142e74ac1c |
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
|  3.9 GB  |     3.454540209     |      /some-pretty-damn-long-sample-container-name        | 61f2cb9dc16d047f42152dde98c00e5a9f9e2a8ba305adfbf38252af3586e07a |
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
| 17.2 GB  |     1.430940032     |                  /sample-container-three                 | 1b05272735bf083e1b148169130f620505dbc2fcff3c4fc6f8f46e49a4efa676 |
+----------+---------------------+----------------------------------------------------------+------------------------------------------------------------------+
```

---

The containers on my machine as i write this:
```
$ sudo ./container-stats
+----------+---------------------+-----------------+------------------------------------------------------------------+
|  memory  | average_percent_cpu |      name       |                                id                                |
+----------+---------------------+-----------------+------------------------------------------------------------------+
|  7.5 MB  |     0.007575758     | /cranky_perlman | 319e4cebf66e3c90e8eed555242f6628fbb6888e862435e4e9be0fd05a5e4c2e |
+----------+---------------------+-----------------+------------------------------------------------------------------+
| 128.8 MB |    0.0152870985     |   /portainer    | 60c914b19bf7d81019d5f3bb28b201297ead82be87b14d96c259c23612d0d515 |
+----------+---------------------+-----------------+------------------------------------------------------------------+
```

... but one of them is secretly a CPU hog! *(`--top` checks all processes in the container, but is also much slower)*
```
$ sudo ./container-stats --top
+----------+---------------------+-----------------+------------------------------------------------------------------+
|  memory  | average_percent_cpu |      name       |                                id                                |
+----------+---------------------+-----------------+------------------------------------------------------------------+
| 22.5 MB  |      199.53616      | /cranky_perlman | 319e4cebf66e3c90e8eed555242f6628fbb6888e862435e4e9be0fd05a5e4c2e |
+----------+---------------------+-----------------+------------------------------------------------------------------+
| 128.8 MB |     0.015279379     |   /portainer    | 60c914b19bf7d81019d5f3bb28b201297ead82be87b14d96c259c23612d0d515 |
+----------+---------------------+-----------------+------------------------------------------------------------------+
```

And in case you want to further process it with a script:
```
$ sudo ./container-stats --top --json
[
  {
    "memory": 22499328,
    "average_percent_cpu": 199.51184,
    "name": "/cranky_perlman",
    "id": "319e4cebf66e3c90e8eed555242f6628fbb6888e862435e4e9be0fd05a5e4c2e"
  },
  {
    "memory": 128811008,
    "average_percent_cpu": 0.015256125,
    "name": "/portainer",
    "id": "60c914b19bf7d81019d5f3bb28b201297ead82be87b14d96c259c23612d0d515"
  }
]
```

---

The total VSZ usage of all containers starting with `/sample-`:
```
$ sudo ./container-stats -t -r "^/sample-"
Total: 106.4 GB (106439839744 B)
```

Show the RSS usage of all containers, combined:
```
$ sudo ./container-stats -m rss -t
Total: 114.3 GB (114328526848 B)
```
*(To learn about the difference between RSS and VSZ, see https://stackoverflow.com/questions/7880784)*

---

All containers, grouped by their prefixes, sorted by memory usage:
```
$ sudo ./container-stats --group-by-prefix -s
+----------+---------------------+------------+------------+
|  memory  | average_percent_cpu | containers |    fix     |
+----------+---------------------+------------+------------+
| 239.3 GB |      2.685421       |     37     |   /fatfix  |
+----------+---------------------+------------+------------+
| 133.1 GB |      18.31371       |     10     |  /prefix1  |
+----------+---------------------+------------+------------+
| 106.4 GB |      17.382954      |     11     |  /prefix2  |
+----------+---------------------+------------+------------+
| 88.3 GB |      6.465111       |     12     |    /fix    |
+----------+---------------------+------------+------------+
| 14.8 GB  |      2.9222221      |     4      |  /boring   |
+----------+---------------------+------------+------------+
| 11.3 GB  |      1.2392704      |     12     |  /foofix   |
+----------+---------------------+------------+------------+
| 10.6 GB  |      3.7951217      |     5      |  /barfix   |
+----------+---------------------+------------+------------+
|  4.3 GB  |      4.833365       |     4      |  /prefix3  |
+----------+---------------------+------------+------------+
| 128.2 MB |     0.019668292     |     1      |  /tinyfix  |
+----------+---------------------+------------+------------+
```
