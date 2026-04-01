/* ================================================================== */
/*  memory.x — FM33A068EV Main固件链接脚本                             */
/*                                                                    */
/*  Normal区: 0x0000_4000 ~ 0x000233FFF (128KB)                     */
/*  OTA区:  0x0002_4000 ~ 0x0004_7FFF (128KB)                     */
/*  Param区: 0x0004_4000 ~ 0x0004_7FFF (16KB)                     */
/*  SRAM: 80KB                                                    */
/*  (c) 2026 FeMeter Project — ViewWay                                */
/* ================================================================== */
MEMORY
{
    FLASH (rx)  : ORIGIN = 0x00004000, LENGTH = 128K
 OTA   (rx)   : ORIGIN = 0x00024000, LENGTH = 128K
 PARAM (rx)  : ORIGIN = 0x00044000, LENGTH = 16K  RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 80K
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
