# MCDU-32 firmware container

The `MCDU-32` sample was stored under SimAppPro's `Download/Firmware` directory.
It is 235,230 bytes long and has SHA-256
`BE75FFFC4A2340008201B34E97BB6E074A0CB75B44EFC4785C7D7521BD34054E`.
The application configures the grid and loads fonts using a separate
[HID display protocol](MCDU-display-capabilities.md). It neither ships nor
installs firmware.

## Container layout

Offsets are zero-based.

| Offset | Length | Value or interpretation |
| ---: | ---: | --- |
| `0x00000` | 8 bytes | Header: `26 01 32 BB 00 01 06 01` |
| `0x00008` | 235,216 bytes | Payload, aligned to 16 bytes |
| `0x396D8` | 4 bytes | `D0 96 03 00`, the payload length in little-endian order |
| `0x396DC` | 2 bytes | Trailer: `42 07`; purpose undetermined |

| Sample | Total bytes | Payload bytes |
| --- | ---: | ---: |
| MCDU-32 | 235,230 | 235,216 |
| FCU1 | 77,102 | 77,088 |
| EFIS_320R | 48,574 | 48,560 |

## Payload observations

Sample entropy is approximately 7.696 bits per byte. No meaningful ASCII
strings of eight or more characters were found. The block
`1A D7 83 22 5A 99 DC 77 BE A3 8D A3 CA 26 36 11` occurs 3,079 times at
16-byte boundaries relative to the payload start.

The observations are consistent with a block transformation but do not identify
AES, ECB or any other algorithm. Neither a simple byte sum nor the tested
CRC-CCITT variants with initial values 0 and 65535 matched the trailer.
The container layout does not establish the processor, executable code,
USB protocol or firmware version. The observations come from file inspection.
