---
triggers: ["RTOS", "FreeRTOS", "Zephyr", "real-time operating system", "task scheduling embedded", "embedded OS"]
tools_allowed: ["read_file", "write_file", "bash"]
category: embedded
---

# Embedded RTOS Development

When working with real-time operating systems:

1. Assign task priorities based on rate-monotonic analysis — give the highest priority to tasks with the shortest period, and use preemptive scheduling so that urgent tasks always interrupt lower-priority work without manual yield calls.
2. Use FreeRTOS APIs correctly — create tasks with xTaskCreate specifying adequate stack depth, pass data between tasks with xQueueSend/xQueueReceive, and protect shared resources with xSemaphoreTake/xSemaphoreGive using mutex semaphores rather than disabling interrupts globally.
3. Leverage Zephyr kernel services including k_thread_create for tasks, k_msgq for message queues, k_sem for counting semaphores, and k_timer for periodic callbacks — use Zephyr's devicetree and Kconfig system to configure hardware and kernel options declaratively.
4. Guard against priority inversion by using mutexes with priority inheritance enabled (the default in FreeRTOS mutexes and Zephyr k_mutex) so that a low-priority task holding a resource temporarily inherits the priority of the highest-priority task waiting on it.
5. Choose the right inter-task communication primitive — use message queues for producer-consumer data flow, event groups or flags for signaling multiple conditions, and direct-to-task notifications in FreeRTOS for lightweight single-value signaling with minimal overhead.
6. Prefer static memory pools (FreeRTOS heap_1 or heap_4 with configTOTAL_HEAP_SIZE, Zephyr K_MEM_POOL_DEFINE) over malloc/free to avoid heap fragmentation — allocate all tasks, queues, and semaphores statically with xTaskCreateStatic or K_THREAD_STACK_DEFINE at compile time.
7. Enable tick-less idle mode (configUSE_TICKLESS_IDLE in FreeRTOS, CONFIG_TICKLESS_KERNEL in Zephyr) to suppress periodic timer interrupts when no tasks are ready, allowing the MCU to remain in deep sleep for extended periods and drastically reducing power consumption.
8. Configure hardware timers to drive the RTOS tick — select a timer with sufficient resolution for your scheduling granularity (typically 1 ms), and ensure the timer interrupt priority is set correctly relative to the RTOS-managed interrupt ceiling (configMAX_SYSCALL_INTERRUPT_PRIORITY).
9. Enable stack overflow detection (configCHECK_FOR_STACK_OVERFLOW set to 2 in FreeRTOS, CONFIG_THREAD_STACK_INFO in Zephyr) during development, and implement the overflow hook to log the offending task name and halt — use high-water-mark APIs (uxTaskGetStackHighWaterMark) to right-size stacks before production.
10. Analyze worst-case execution time (WCET) for all hard-real-time tasks using measurement (cycle counters, DWT on Cortex-M) or static analysis tools, and verify that the total CPU utilization stays below the schedulability bound to guarantee all deadlines are met.
11. Understand the difference between a mutex and a binary semaphore — use mutexes for mutual exclusion of shared resources (they support priority inheritance and ownership), and use binary semaphores for synchronization and signaling between tasks or from ISRs to tasks.
12. Port an RTOS to new hardware by implementing the minimal platform layer — provide a tick timer ISR, a context-switch function (PendSV handler on Cortex-M), stack initialization for new tasks, and critical section enter/exit macros — then validate with a blinking LED task before layering on drivers and application logic.
