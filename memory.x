MEMORY {
    /* 4 MB di Flash sul Pico 2 / Pico 2 W */
    FLASH : ORIGIN = 0x10000000, LENGTH = 4096K
    /* 512 KB di SRAM principale contigua */
    RAM   : ORIGIN = 0x20000000, LENGTH = 512K
    /* Banchi SRAM8 e SRAM9 da 4KB ciascuno */
    SRAM8 : ORIGIN = 0x20080000, LENGTH = 4K
    SRAM9 : ORIGIN = 0x20081000, LENGTH = 4K
}

SECTIONS {
    /* Blocco Image Definition richiesto dal bootrom dell'RP2350 nei primi 4 KB */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .;
        KEEP(*(.start_block));
        KEEP(*(.boot_info));
    } > FLASH
} INSERT AFTER .vector_table;

/* Sposta .text subito dopo .start_block */
_stext = ADDR(.start_block) + SIZEOF(.start_block);

SECTIONS {
    /* Metadati Picotool (Binary Info) */
    .bi_entries : ALIGN(4)
    {
        KEEP(*(.bi_entries));
        . = ALIGN(4);
    } > FLASH
} INSERT AFTER .text;

SECTIONS {
    /* Blocco finale per la validazione della ROM */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH
} INSERT AFTER .uninit;

PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr);