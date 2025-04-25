#![no_std]
#![no_main]

use core::fmt::Write;
use hal::{
    entry,
    gpio::{self, FunctionPio0, Pin},
    pio::{PIOBuilder, PIOExt, PinDir, ShiftDirection},
    uart::{DataBits, StopBits, UartConfig},
    {Sio, pac},
};
use panic_halt as _;
use rp2040_hal as hal;
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::{Clock, clocks};

#[unsafe(link_section = ".boot_loader")]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    // Configure the clocks
    let clocks = clocks::init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let pin0: Pin<_, FunctionPio0, _> = pins.gpio0.into_function();
    let pin1: Pin<_, FunctionPio0, _> = pins.gpio1.into_function();
    let data_pin_id = pin0.id().num;
    let debug_pin_id = pin1.id().num;

    #[cfg(not(feature = "bidirectional"))]
    let program = pio_proc::pio_asm!(
        ".wrap_target",
        "set x, 1"
        "reset:",
        "  set y, 200"
        "  wait 0 pin 0",
        "loop:",
        "  jmp pin reset",
        "  jmp y-- loop"
        "begin:",
        "  set pins, 0",
        "  wait 0 pin 0",
        "  wait 1 pin 0",
        "  nop [5]"
        "  jmp pin one",
        "zero:",
        "  in null, 1",
        "  jmp begin",
        "one:",
        "  set pins, 1"
        "  in x, 1",
        "  jmp begin",
        ".wrap"
    );

    #[cfg(feature = "bidirectional")]
    let program = pio_proc::pio_asm!(
        ".wrap_target",
        "set x, 1"
        "begin:",
        "  set y, 20"
        "  wait 0 pin 0",
        "watch:",
        "  jmp pin begin",
        "  jmp y-- watch",
        "  set y, 15"
        "  wait 1 pin 0",
        "capture:",
        "  wait 0 pin 0",
        "  set pins, 0"
        "  nop [15]"
        "  jmp pin zero",
        "one:",
        "  set pins, 1",
        "  in x, 1",
        "  wait 1 pin 0",
        "  jmp latch",
        "zero:",
        "  in null, 1",
        "latch:"
        "  jmp y-- capture",
        ".wrap"
    );

    // Initialize and start PIO
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();

    // DShot 300 requires a double frequency of the producer.
    // So the clock divisor of the producer is `(50, 0)` we need to use `(25, 0)` here.
    //
    let (int, frac) = (10, 0);
    let (mut sm, mut rx, _) = PIOBuilder::from_installed_program(installed)
        .set_pins(debug_pin_id, 1)
        .jmp_pin(data_pin_id)
        .autopush(true)
        .push_threshold(16)
        .clock_divisor_fixed_point(int, frac)
        .in_shift_direction(ShiftDirection::Left)
        .build(sm0);
    sm.set_pindirs([(debug_pin_id, PinDir::Output)]);
    sm.start();

    let uart_pins = (
        // UART TX on pin 4
        pins.gpio4.into_function(),
        // UART RX on pin 5
        pins.gpio5.into_function(),
    );
    let mut uart = hal::uart::UartPeripheral::new(pac.UART1, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    uart.write_full_blocking(b"UART is started\r\n");

    let mut val: u32 = 0;

    loop {
        if let Some(value) = rx.read() {
            if value == val {
                continue;
            }
            val = value;
            let crc = value & 0b1111;
            let throttle = value >> 4;
            let expected_crc = !(throttle ^ (throttle >> 4) ^ (throttle >> 8)) & 0b1111;
            let crc_ok = crc == expected_crc;
            let telemetry = throttle & 1;
            let throttle = throttle >> 1;
            writeln!(uart, "value: {value:#b}\r\n").unwrap();
            writeln!(
                uart,
                "throttle: {throttle}\r\ncrc: {crc}  ok: {crc_ok:?}\r\ntm: {telemetry}\r\n"
            )
            .unwrap();
        }
    }
}
