# TCA6408A demo for the Wokwi web editor

wokwi.com no longer compiles Rust online, so this demo uses an Arduino sketch. It speaks
raw I²C to the same TCA6408A chip, reads the buttons and drives the `VCI_EN` LED, and
prints in the same format as the [Rust example](../esp32c3). The chip is loaded from this
repository's GitHub Release via the `dependencies` entry in `diagram.json`.

Live project: <TODO>

To recreate it on wokwi.com:

1. Start a new **ESP32-C3** Arduino project.
2. Replace `sketch.ino` and `diagram.json` with the files from this directory.
3. Start the simulation. The TCA6408A chip has no controls: press the BOT, UP, LEFT, DOWN
   and RIGHT buttons wired to P0 to P4. The green LED on P7 shows `VCI_EN`.
