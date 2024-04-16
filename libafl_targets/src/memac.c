#include <stdint.h>
#include "common.h"
#include <stdio.h>

extern uint8_t __mem_ac_ptr_local[MEM_MAP_SIZE * 2];
uint8_t *__input_start;
uint8_t *__input_end;
void __libafl_hook_read(uint64_t addr) {
    if(addr >= (uint64_t)__input_start && addr <= (uint64_t) __input_end) {
        return;
    }
    uint64_t pos = addr % MEM_MAP_SIZE;
    __mem_ac_ptr_local[pos] += 1;
}

void __libafl_hook_write(uint64_t addr) {
    if(addr >= (uint64_t)__input_start && addr <= (uint64_t) __input_end) {
        return;
    }
    uint64_t pos = addr % MEM_MAP_SIZE;
    // printf("write %lu", addr);
    __mem_ac_ptr_local[pos + MEM_MAP_SIZE] += 1; 
}