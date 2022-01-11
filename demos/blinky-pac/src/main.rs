#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use stm32_metapac as pac;

use pac::gpio::{self, vals};

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, World!");

    let gpioc = pac::GPIOC;
    const BUTTON_PIN: usize = 13;
    // Setup button
    unsafe {
        gpioc
            .pupdr()
            .modify(|w| w.set_pupdr(BUTTON_PIN, vals::Pupdr::PULLUP));
        gpioc
            .otyper()
            .modify(|w| w.set_ot(BUTTON_PIN, vals::Ot::PUSHPULL));
        gpioc
            .moder()
            .modify(|w| w.set_moder(BUTTON_PIN, vals::Moder::INPUT));
    }

    // Setup LED
    let gpioa = pac::GPIOA;
    const LED_PIN: usize = 5;
    unsafe {
        gpioa
            .pupdr()
            .modify(|w| w.set_pupdr(LED_PIN, vals::Pupdr::FLOATING));
        gpioa
            .otyper()
            .modify(|w| w.set_ot(LED_PIN, vals::Ot::PUSHPULL));
        gpioa
            .moder()
            .modify(|w| w.set_moder(LED_PIN, vals::Moder::OUTPUT));
    }

    // Main loop
    loop {
        unsafe {
            if gpioc.idr().read().idr(BUTTON_PIN) == vals::Idr::LOW {
                gpioa.bsrr().write(|w| w.set_bs(LED_PIN, true));
            } else {
                gpioa.bsrr().write(|w| w.set_br(LED_PIN, true));
            }
        }
    }
}
