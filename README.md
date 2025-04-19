## DShot reader test

This binary crate is a minitest for DShot protocol communication.

For the moment it is tuned for "listening" to DShot 300.

The received messages are decoded 
and can be observed as human-readable logs through a serial terminal.

### Hardware

On the "publisher side" a sender of DShot data signal
with level `3.3V` is required.

On the "consumer side" one RP2040 and one UART USB adapter 
are required to run the program and read the logs.

### Connection

- connect RP2040 GPIO 4 pin to RX pin on the UART adapter;
- connect RP2040 signal ground to the ground of UART adapter;
- connect RP2040 GPIO 0 to the source of DShot signals;
- connect RP2040 signal ground to the ground of DSHot source;

### Installation

The compiled binary could be installed to PR2040 when it's
connected to the host computer as USB mass storage.
Then run `cargo r`.

### Debugging

The signals on GPIOs 0 and 1 could be compared on oscilloscope.

Debug-signal on the pin GPIO 1 pulls shortly up and down
when the logical "1" on the source sequence is detected.
