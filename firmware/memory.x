/* memory.x — FM33A068EV 链接器内存布局 */

MEMORY
{
  /* Flash: 512KB */
  FLASH (rx)  : ORIGIN = 0x00000000, LENGTH = 512K

  /* SRAM: 80KB */
  RAM (rwx)   : ORIGIN = 0x20000000, LENGTH = 80K
}
