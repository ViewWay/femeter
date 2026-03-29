MEMORY
{
    /* Bootloader 从 0x0000_0000 开始, 占 16KB */
    FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 16K
    RAM (rwx)  : ORIGIN = 0x20000000, LENGTH = 80K
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
