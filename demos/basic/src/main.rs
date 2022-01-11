#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

#[allow(unused_imports)]
use stm32l4::stm32l4x5 as pac;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, World!");
    loop {}
}
