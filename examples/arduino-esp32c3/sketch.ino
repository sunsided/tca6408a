// TCA6408A I/O expander demo for the Wokwi web editor (Arduino, ESP32-C3).
//
// Talks to the TCA6408A Wokwi chip over raw I2C, reads buttons and drives a supply enable,
// printing in the same format as the Rust example in ../esp32c3.
// Wiring: SDA = GPIO4, SCL = GPIO5, INT = GPIO6, RESET = GPIO7; ADDR low (0x20).
//
// Expander pins: P0 BOT, P1 up, P2 left, P3 down and P4 right buttons (active low),
// P5 IMU INT1, P6 touch IRQ, P7 output (VCI_EN panel supply, the LED).

#include <Wire.h>

constexpr uint8_t TCA6408A_ADDRESS = 0x20;

constexpr uint8_t REG_INPUT_PORT = 0x00;
constexpr uint8_t REG_OUTPUT_PORT = 0x01;
constexpr uint8_t REG_CONFIGURATION = 0x03;

// The VCI_EN panel supply enable.
constexpr uint8_t VCI_EN = 1 << 7;

// The active-low buttons on P0 to P4.
struct Button {
  const char *name;
  uint8_t mask;
};

constexpr Button BUTTONS[] = {
    {"BOT", 1 << 0}, {"UP", 1 << 1}, {"LEFT", 1 << 2}, {"DOWN", 1 << 3}, {"RIGHT", 1 << 4},
};

constexpr int PIN_SDA = 4;
constexpr int PIN_SCL = 5;
constexpr int PIN_INT = 6;
constexpr int PIN_RST = 7;

bool readRegister(uint8_t reg, uint8_t *value) {
  Wire.beginTransmission(TCA6408A_ADDRESS);
  Wire.write(reg);
  if (Wire.endTransmission(false) != 0) {
    return false;
  }

  if (Wire.requestFrom(int(TCA6408A_ADDRESS), 1) != 1) {
    return false;
  }

  *value = Wire.read();
  return true;
}

bool writeRegister(uint8_t reg, uint8_t value) {
  Wire.beginTransmission(TCA6408A_ADDRESS);
  Wire.write(reg);
  Wire.write(value);
  return Wire.endTransmission() == 0;
}

void printInputs(uint8_t inputs) {
  Serial.printf("inputs=0x%02X vci_en=%u\n", inputs, (inputs & VCI_EN) != 0);

  for (const Button &button : BUTTONS) {
    if ((inputs & button.mask) == 0) {
      Serial.printf("pressed %s\n", button.name);
    }
  }
}

void setup() {
  Serial.begin(115200);

  // Reset: RESET low for at least 1 µs.
  pinMode(PIN_RST, OUTPUT);
  digitalWrite(PIN_RST, LOW);
  delayMicroseconds(1);
  digitalWrite(PIN_RST, HIGH);

  pinMode(PIN_INT, INPUT_PULLUP);
  Wire.begin(PIN_SDA, PIN_SCL);
  Wire.setClock(400000);

  // The latch goes high before P7 becomes an output, so the supply never glitches off.
  uint8_t output = 0, config = 0;
  if (!readRegister(REG_OUTPUT_PORT, &output) || !readRegister(REG_CONFIGURATION, &config) ||
      !writeRegister(REG_OUTPUT_PORT, output | VCI_EN) ||
      !writeRegister(REG_CONFIGURATION, config & ~VCI_EN)) {
    Serial.println("TCA6408A not found");
    while (true) {
      delay(1000);
    }
  }

  readRegister(REG_CONFIGURATION, &config);
  readRegister(REG_OUTPUT_PORT, &output);
  Serial.printf("TCA6408A ok config=0x%02X output=0x%02X\n", config, output);
}

void loop() {
  // Reading the inputs clears INT.
  uint8_t inputs = 0;
  if (readRegister(REG_INPUT_PORT, &inputs)) {
    printInputs(inputs);
  } else {
    Serial.println("read failed");
  }

  // INT goes low when an input changes.
  while (digitalRead(PIN_INT) == HIGH) {
  }
}
