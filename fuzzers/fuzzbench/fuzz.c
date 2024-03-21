#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

void f(const uint8_t *Data) {
  if (Data[1] == 'o') {
    printf("2nd");
  }
}

int LLVMFuzzerTestOneInput(const uint8_t *Data, size_t Size) {
  if (Size >= 8 && *(uint32_t *)Data == 0xaabbccdd) { 
    printf("aborting!");
  }
  char buf[8] = {'a', 'b', 'c', 'd'};

  if (memcmp(Data, buf, 4) == 0) { 
	  printf("Hellow\n");
  }
  char *ptr = malloc(8);
  if (Size < 8) {
    return 0;
  }
  if (Data[0] == 'j') {
      char uninitialized = ptr[3];
      printf("1st");
      printf("%c\n", uninitialized);
      f(Data);
  }
  return 0;
}

/*
int main() {

  char buf [10] = {0};
  LLVMFuzzerTestOneInput(buf, 10);

}*/
