/* ================================================================== */
/*                                                                    */
/*  memory.x — FM33A068EV Linker Script                               */
/*                                                                    */
/*  512KB Flash, 80KB SRAM                                            */
/*  分区: Boot(16KB) + Normal(128KB) + OTA(128KB) + Param(16KB)       */
/*                                                                    */
/*  (c) 2026 FeMeter Project                                          */
/* ================================================================== */

MEMORY
{
    /* Bootloader 区: 16KB (0x0000_0000 ~ 0x0000_3FFF) */
    BOOT (rx)  : ORIGIN = 0x00000000, LENGTH = 16K

    /* 应用程序 Normal 区: 128KB (0x0000_4000 ~ 0x0002_3FFF) */
    FLASH (rx) : ORIGIN = 0x00004000, LENGTH = 128K

    /* OTA 备份区: 128KB (0x0002_4000 ~ 0x0004_3FFF) */
    OTA (rx)   : ORIGIN = 0x00024000, LENGTH = 128K

    /* 参数存储区: 16KB (0x0004_4000 ~ 0x0004_7FFF)
     * 剩余 Flash: 512K - 16K - 128K - 128K - 16K = 224K (保留) */
    PARAM (rx) : ORIGIN = 0x00044000, LENGTH = 16K

    /* SRAM: 80KB */
    RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 80K
}

/* Bootloader 使用的 linker — 起始地址 = 0x00000000 */
/* 正常固件使用的 linker — 起始地址 = 0x00004000 */

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
