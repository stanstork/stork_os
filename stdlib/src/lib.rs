#![no_std]
#![feature(lang_items)] // Required for custom std

pub mod alloc;
pub mod io;
pub mod sys;

#[macro_use]
pub mod macros;
