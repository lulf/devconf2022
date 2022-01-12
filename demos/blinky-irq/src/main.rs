#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use embassy_stm32::{
    gpio::{Input, Level, Output, Pin, Pull, Speed},
    interrupt, pac,
    peripherals::{PB14, PC13},
};
use embedded_hal::digital::v2::{InputPin, OutputPin};

const BUTTON_PIN: usize = 13;
const BUTTON_PORT: u8 = 2;

static mut BUTTON: Option<Input<'static, PC13>> = None;
static mut LED: Option<Output<'static, PB14>> = None;

#[entry]
fn main() -> ! {
    let p = embassy_stm32::init(Default::default());
    let led = p.PB14;
    let button = p.PC13;

    // Setup interrupt
    cortex_m::interrupt::free(|_| unsafe {
        LED.replace(Output::new(led, Level::Low, Speed::VeryHigh));
        BUTTON.replace(Input::new(button, Pull::Up));

        NVIC::unpend(pac::Interrupt::EXTI15_10);
        NVIC::unmask(pac::Interrupt::EXTI15_10);
    });

    enable_button();

    loop {
        //cortex_m::asm::wfe();
    }
}

fn enable_button() {
    let pin = BUTTON_PIN;
    let port = BUTTON_PORT;
    let syscfg = pac::SYSCFG;
    let exti = pac::EXTI;
    cortex_m::interrupt::free(|_| unsafe {
        syscfg.exticr(pin / 4).modify(|w| w.set_exti(pin % 4, port));
        exti.rtsr(0).modify(|w| w.set_line(pin, true));
        exti.ftsr(0).modify(|w| w.set_line(pin, true));

        exti.pr(0).write(|w| w.set_line(pin, true));
        exti.imr(0).modify(|w| w.set_line(pin, true));
    });
}

#[interrupt]
fn EXTI15_10() {
    let exti = pac::EXTI;
    unsafe {
        defmt::info!("EXTI15_10 IRQ");
        let mut lines = exti.pr(0).read();
        if lines.line(BUTTON_PIN) {
            defmt::info!("BUTTON PIN LINE SET");
            let imr = exti.imr(0).read();
            if !imr.line(BUTTON_PIN) {
                if BUTTON.as_ref().unwrap().is_low().unwrap() {
                    LED.as_mut().unwrap().set_high().unwrap();
                } else {
                    LED.as_mut().unwrap().set_low().unwrap();
                }
                defmt::println!("Interrupt from button");
                //unsafe { exti.imr(0).modify(|w| w.set_line(13, false)) };
            }
            lines.set_line(BUTTON_PIN, false);
        }

        exti.pr(0).write_value(lines);
        NVIC::unpend(pac::Interrupt::EXTI15_10);
    }
}
