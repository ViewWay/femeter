/* ================================================================== */
/*  memory-boot.x — Bootloader 专用链接脚本内存定义                    */
/*                                                                    */
/*  Bootloader: 0x0000_0000 ~ 0x0000_3FFF (16KB)                      */
/*  SRAM: 80KB                                                     */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */

MEMORY
{
    FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 16K
 RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 80K
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
