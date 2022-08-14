use core::fmt::Write;
use core::marker::PhantomData;

static mut UART: Option<Uart<0x10000000, Init>> = None;

pub fn initialize() {
    unsafe {assert!(UART.is_none())}
    let uart = unsafe { Uart::<0x10000000, Uninit>::new().init() };
    unsafe { UART = Some(uart) };
}

pub fn get_uart() -> &'static Uart<0x10000000, Init> {
    unsafe { UART.as_ref().unwrap() }
}

pub struct Uninit {}
pub struct Init {}

#[non_exhaustive]
pub struct Uart<const B: usize, S = Uninit>(PhantomData<S>, PhantomData<*mut ()>);

impl<const B: usize> Uart<B, Uninit> {
    pub fn new() -> Uart<B, Uninit> {
        Uart(PhantomData::default(), PhantomData::default())
    }
    pub unsafe fn init(self) -> Uart<B, Init> {
        uart_init(B);
        Uart(PhantomData::default(), PhantomData::default())
    }
}
impl<const B: usize> Uart<B, Init> {
    pub fn get(&self) -> Option<u8> {
        uart_get(B)
    }
    pub fn put(&self, c: u8) {
        uart_put(B, c)
    }
}

impl<const B: usize> Default for Uart<B, Uninit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const B: usize> Write for &Uart<B, Init> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.put(c)
        }
        Ok(())
    }
}

fn uart_init(base_addr: usize) {
    let ptr = base_addr as *mut u8;
    unsafe {
        let lcr = 0b11;
        ptr.add(3).write_volatile(lcr);
        ptr.add(2).write_volatile(0b1);
        ptr.add(1).write_volatile(0b1);

        //divisor
        let divisor: u16 = 592;
        let div_bytes = divisor.to_be_bytes();

        ptr.add(3).write_volatile(lcr | 1 << 7);
        ptr.add(0).write_volatile(div_bytes[1]);
        ptr.add(1).write_volatile(div_bytes[0]);
        ptr.add(3).write_volatile(lcr);
    }
}

fn uart_get(base_addr: usize) -> Option<u8> {
    let ptr = base_addr as *mut u8;
    unsafe {
        if ptr.add(5).read_volatile() & 1 == 0 {
            None
        } else {
            Some(ptr.read_volatile())
        }
    }
}

fn uart_put(base_addr: usize, c: u8) {
    let ptr = base_addr as *mut u8;
    unsafe {
        ptr.add(0).write_volatile(c);
    }
}
