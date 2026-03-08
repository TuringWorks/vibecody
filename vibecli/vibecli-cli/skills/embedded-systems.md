---
triggers: ["embedded systems", "firmware", "microcontroller", "MCU", "bare metal", "embedded C", "ARM Cortex"]
tools_allowed: ["read_file", "write_file", "bash"]
category: embedded
---

# Embedded Systems Development

When working with embedded systems and firmware:

1. Write C/C++ with resource constraints in mind — avoid dynamic allocation on heap, prefer static buffers, and use fixed-size data types (uint8_t, uint32_t) from stdint.h to guarantee memory layout across compilers and architectures.
2. Use memory-mapped I/O with volatile-qualified pointers to access hardware registers, and define register maps as structs with explicit padding to match the peripheral memory layout documented in the datasheet.
3. Keep interrupt service routines (ISRs) as short as possible — set a flag or enqueue data into a ring buffer, then handle the work in the main loop or a deferred task to avoid blocking other interrupts and missing deadlines.
4. Configure DMA channels for efficient data transfer between peripherals (UART, SPI, ADC) and memory, freeing the CPU to perform computation or enter low-power states while bulk transfers complete in the background.
5. Implement power management by using sleep modes (WFI/WFE on ARM Cortex) aggressively, gating unused peripheral clocks, and waking only on relevant interrupts or timer events to minimize current draw in battery-powered designs.
6. Enable and service a watchdog timer to recover from firmware hangs — feed the watchdog only from verified healthy code paths, and log the reset reason on boot to aid post-mortem debugging of field failures.
7. Design a robust bootloader that validates firmware images (CRC32 or SHA-256) before jumping to the application, supports A/B partition schemes for safe OTA updates, and provides a fallback mechanism if the new image fails to boot.
8. Build a hardware abstraction layer (HAL) that isolates platform-specific register access behind clean C interfaces, enabling the same application logic to compile and run on different MCU families (STM32, nRF, ESP32) with only the HAL swapped out.
9. Use JTAG or SWD debug probes (J-Link, ST-Link, CMSIS-DAP) with GDB for step-through debugging, breakpoint setting, and live memory inspection — supplement with logic analyzers or oscilloscopes to correlate firmware behavior with electrical signals.
10. Define the memory layout explicitly in a linker script — place code in flash, initialized data in RAM with flash-backed load regions, reserve stack and heap sizes, and use linker-generated symbols to initialize .bss and .data sections at startup.
11. Set up cross-compilation toolchains (arm-none-eabi-gcc, LLVM) with the correct target triple, CPU flags (-mcpu=cortex-m4 -mfloat-abi=hard), and optimization level (-Os for size, -O2 for speed), and automate builds with CMake or Makefiles that produce .elf, .bin, and .hex artifacts.
12. Unit test embedded code on the host using frameworks like Unity or CppUTest — mock hardware dependencies behind the HAL interface, run tests in CI with sanitizers enabled, and use code coverage tools to identify untested paths before flashing to real hardware.
