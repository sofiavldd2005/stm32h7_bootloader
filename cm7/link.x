
EXTERN(RESET_VECTOR);

MEMORY
{
  FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 2M
  RAM (rwx)   : ORIGIN = 0x20000000, LENGTH = 128K
}


/* vector table at 0x08000000 with SP = 0x20020000 (top of DTCM)
and the Reset handler pointer second. .text follows immediately after.*/
SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    LONG(ORIGIN(RAM) + LENGTH(RAM));
    KEEP(*(.vector_table.reset_vector));
  } > FLASH

  .text :
  {
    *(.text .text.*);
    *(.rodata .rodata.*);
  } > FLASH

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
  }
}
