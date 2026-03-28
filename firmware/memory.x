MEMORY
{
  /* FM33LG0xx: 256KB Flash */
  FLASH  : ORIGIN = 0x00000000, LENGTH = 256K
  /* FM33LG0xx: 32KB RAM (divided) */
  RAM    : ORIGIN = 0x20000000, LENGTH = 24K
  /* Battery-backed RAM for RTC / backup registers */
  BKP    : ORIGIN = 0x20006000, LENGTH = 8K
}
