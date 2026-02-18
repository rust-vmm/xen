/* Copyright (C) 1991-2020 Free Software Foundation, Inc.
   This file is part of the GNU C Library.

   The GNU C Library is free software; you can redistribute it and/or
   modify it under the terms of the GNU Lesser General Public
   License as published by the Free Software Foundation; either
   version 2.1 of the License, or (at your option) any later version.

   The GNU C Library is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
   Lesser General Public License for more details.

   You should have received a copy of the GNU Lesser General Public
   License along with the GNU C Library; if not, see
   <https://www.gnu.org/licenses/>.  */
OUTPUT_ARCH(aarch64)
OUTPUT_FORMAT("elf64-littleaarch64")

MEMORY
{
	RAM : ORIGIN = 0x60000000, LENGTH = 512M
}

ENTRY(_start)
SECTIONS
{
    . = 0x60000000;
    _text = .;
    .text : {
        *(.text)
    } > RAM
    .text.other : {
	*(.text.*)
    } > RAM
    . = ALIGN((1 << 12));
    .data : {
        *(.data)
    . = ALIGN((1 << 12));
        *(.data.page_aligned)
    } > RAM
    . = ALIGN((1 << 12));
    .rodata : {
        *(.rodata)
        *(.rodata.*)
    } > RAM
    . = ALIGN((1 << 12));
    .bss : {
        __start_bss = .;
        *(.bss)
    . = ALIGN((1 << 12));
        *(.bss.page_aligned)
        __end_bss = .;
    } > RAM
    _end = .;
    /DISCARD/ :
    {
	*(.ARM.exidx .ARM.exidx.*)
    }
}
