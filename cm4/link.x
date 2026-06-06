ENTRY(Reset);
EXTERN(RESET_VECTOR);
EXTERN(EXCEPTIONS);

MEMORY
{
  FLASH (rx)  : ORIGIN = 0x08100000, LENGTH = 1M
  RAM (rwx)   : ORIGIN = 0x10000000, LENGTH = 128K
}

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));
    KEEP(*(.vector_table.reset_vector));
    KEEP(*(.vector_table.exceptions));
  } > FLASH

  .text :
  {
    *(.text .text.*);
    *(.rodata .rodata.*);
  } > FLASH
/*reserve the last 4 bytes of CM4 flash for a CRC32:
 * */
  .crc ORIGIN(FLASH) + LENGTH(FLASH) - 4 :
  {
    LONG(0x00000000);
  } > FLASH

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}

PROVIDE(NMI = DefaultExceptionHandler);
PROVIDE(HardFault = DefaultExceptionHandler);
PROVIDE(MemManage = DefaultExceptionHandler);
PROVIDE(BusFault = DefaultExceptionHandler);
PROVIDE(UsageFault = DefaultExceptionHandler);
PROVIDE(SVCall = DefaultExceptionHandler);
PROVIDE(PendSV = DefaultExceptionHandler);
PROVIDE(SysTick = DefaultExceptionHandler);
