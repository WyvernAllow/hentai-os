#![no_main]
#![no_std]

use uefi::{prelude::*, println};

#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    system_table
        .boot_services()
        .set_watchdog_timer(0, 0, None)
        .expect("Failed to set watchdog timer.");

    println!("Welcome to Hentai OS!");

    for i in (1..=5).rev() {
        println!("Exiting in {i} seconds...");
        system_table.boot_services().stall(1_000_000);
    }

    Status::SUCCESS
}
