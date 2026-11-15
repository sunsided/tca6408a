# TCA6408A Wokwi custom chip

Wraps the [`tca6408a-sim`](../../tca6408a-sim) behavioral model into a
[Wokwi custom chip](https://docs.wokwi.com/chips-api/getting-started), so it can stand in
for a real TCA6408A in Wokwi projects.

```sh
./build.sh   # produces tca6408a.chip.wasm next to tca6408a.chip.json
```

Reference it from a project's `wokwi.toml`:

```toml
[[chip]]
name = 'tca6408a'
binary = '../../sim/wokwi-chip/tca6408a.chip.wasm'
```

and add a `chip-tca6408a` part to `diagram.json`.

Projects outside this repository (on wokwi.com, in VS Code or with `wokwi-cli`) can use
the `chip.zip` attached to each GitHub Release instead of building the chip. Add a
`chip-tca6408a` part and a dependency to `diagram.json`; no `[[chip]]` entry is needed:

```json
"dependencies": { "chip-tca6408a": "github:sunsided/tca6408a@0.1.0" }
```

The version is given without the `v` prefix of the release tag; `@v0.1.0` fails to load.

The pins follow the TSSOP-16 pinout.

| Pin            | Function                                                    |
|----------------|-------------------------------------------------------------|
| `SDA`          | I²C data                                                    |
| `SCL`          | I²C clock                                                   |
| `ADDR`         | Address select: low or unconnected `0x20`, high `0x21`      |
| `INT`          | Interrupt output, open drain, active low                    |
| `RESET`        | Reset input, active low (pulled up)                         |
| `P0`–`P7`      | Port pins                                                   |
| `VCCI`, `VCCP`, `GND` | Power; not modelled                                  |

Differences from the real part:

- Inputs have pull-ups, standing in for the external pull-ups the datasheet asks for. The
  real inputs are high impedance.
- `ADDR` is sampled once, when the simulation starts.
- `INT` and the outputs change immediately; the real part needs up to 4 µs (`tiv`) and
  400 ns (`tpv`).
